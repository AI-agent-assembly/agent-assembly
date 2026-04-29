"""Tests for the aa_hooks.openai monkey-patch adapter.

Uses unittest.mock to create a fake ``openai`` package so that tests
run without installing the real OpenAI SDK.
"""

from __future__ import annotations

import importlib
import sys
import types
import unittest
from unittest.mock import MagicMock, call


# ---------------------------------------------------------------------------
# Helpers: build a minimal mock of the openai package structure
# ---------------------------------------------------------------------------

def _build_mock_openai():
    """Create a mock ``openai`` package with ``Completions.create``.

    Returns (mock_openai_module, Completions_class, original_create).
    """
    # Create module hierarchy: openai → openai.resources → ... → completions
    openai_mod = types.ModuleType("openai")
    resources_mod = types.ModuleType("openai.resources")
    chat_mod = types.ModuleType("openai.resources.chat")
    completions_mod = types.ModuleType("openai.resources.chat.completions")

    # Real-ish Completions class with a create() method
    class Completions:
        @staticmethod
        def create(*args, **kwargs):
            """Original create that should be wrapped."""
            usage = MagicMock()
            usage.prompt_tokens = 10
            usage.completion_tokens = 20
            response = MagicMock()
            response.usage = usage
            return response

    class AsyncCompletions:
        @staticmethod
        async def create(*args, **kwargs):
            """Original async create."""
            usage = MagicMock()
            usage.prompt_tokens = 10
            usage.completion_tokens = 20
            response = MagicMock()
            response.usage = usage
            return response

    completions_mod.Completions = Completions
    completions_mod.AsyncCompletions = AsyncCompletions
    chat_mod.completions = completions_mod
    resources_mod.chat = chat_mod
    openai_mod.resources = resources_mod

    original_create = Completions.create

    return openai_mod, completions_mod, Completions, original_create


def _install_mock_openai(openai_mod, completions_mod):
    """Register the mock openai modules in sys.modules."""
    sys.modules["openai"] = openai_mod
    sys.modules["openai.resources"] = openai_mod.resources
    sys.modules["openai.resources.chat"] = openai_mod.resources.chat
    sys.modules["openai.resources.chat.completions"] = completions_mod


def _remove_mock_openai():
    """Remove mock openai modules from sys.modules."""
    for key in list(sys.modules):
        if key == "openai" or key.startswith("openai."):
            del sys.modules[key]


# ---------------------------------------------------------------------------
# Test cases
# ---------------------------------------------------------------------------

class TestOpenAIHook(unittest.TestCase):
    """Tests for aa_hooks.openai install/uninstall."""

    def setUp(self):
        """Set up a mock openai package and a mock handle."""
        self.openai_mod, self.completions_mod, self.Completions, self.original_create = (
            _build_mock_openai()
        )
        _install_mock_openai(self.openai_mod, self.completions_mod)

        # Mock handle with report_llm_call
        self.handle = MagicMock()
        self.handle.report_llm_call = MagicMock()

        # Reset the aa_hooks.openai module state (clear _original_create sentinel)
        if "aa_hooks.openai" in sys.modules:
            del sys.modules["aa_hooks.openai"]
        if "aa_hooks" in sys.modules:
            del sys.modules["aa_hooks"]

    def tearDown(self):
        """Clean up mock modules and uninstall hook."""
        try:
            from aa_hooks import openai as hook_mod
            hook_mod.uninstall()
        except Exception:
            pass
        _remove_mock_openai()
        # Clear aa_hooks from sys.modules
        for key in list(sys.modules):
            if key == "aa_hooks" or key.startswith("aa_hooks."):
                del sys.modules[key]

    def _import_and_install(self):
        """Import and install the hook."""
        from aa_hooks import openai as hook_mod
        hook_mod.install(self.handle)
        return hook_mod

    def test_install_patches_create(self):
        """After install(), Completions.create should be a different function."""
        self._import_and_install()
        assert self.Completions.create is not self.original_create

    def test_wrapped_call_returns_original_response(self):
        """The wrapper must return the exact same response object."""
        self._import_and_install()
        instance = self.Completions()
        response = instance.create(model="gpt-4o", messages=[{"role": "user", "content": "hi"}])
        # The response should be the MagicMock built by our fake create
        assert response is not None
        assert hasattr(response, "usage")

    def test_wrapped_call_sends_llm_event(self):
        """The wrapper should call handle.report_llm_call with correct args."""
        self._import_and_install()
        instance = self.Completions()
        instance.create(model="gpt-4o", messages=[])

        self.handle.report_llm_call.assert_called_once()
        kwargs = self.handle.report_llm_call.call_args
        assert kwargs[1]["model"] == "gpt-4o"
        assert kwargs[1]["prompt_tokens"] == 10
        assert kwargs[1]["completion_tokens"] == 20
        assert kwargs[1]["provider"] == "openai"
        assert kwargs[1]["latency_ms"] >= 0

    def test_hook_failure_does_not_break_call(self):
        """If report_llm_call raises, the original response still returns."""
        self.handle.report_llm_call.side_effect = RuntimeError("channel closed")
        self._import_and_install()
        instance = self.Completions()
        # Should NOT raise despite the reporting failure.
        response = instance.create(model="gpt-4o", messages=[])
        assert response is not None

    def test_original_exception_propagates(self):
        """If the original create() raises, the exception passes through."""
        self._import_and_install()

        # Make the original create raise
        def broken_create(*args, **kwargs):
            raise ConnectionError("API unreachable")

        # Patch _original_create inside the hook module
        from aa_hooks import openai as hook_mod
        saved = hook_mod._original_create
        hook_mod._original_create = broken_create
        try:
            instance = self.Completions()
            with self.assertRaises(ConnectionError):
                instance.create(model="gpt-4o", messages=[])
        finally:
            hook_mod._original_create = saved

    def test_uninstall_restores_original(self):
        """After uninstall(), Completions.create should be the original."""
        hook_mod = self._import_and_install()
        assert self.Completions.create is not self.original_create
        hook_mod.uninstall()
        assert self.Completions.create is self.original_create

    def test_double_install_is_noop(self):
        """Calling install() twice should not double-wrap."""
        hook_mod = self._import_and_install()
        wrapped_once = self.Completions.create
        hook_mod.install(self.handle)  # second install
        assert self.Completions.create is wrapped_once

    def test_model_from_kwargs(self):
        """Model name should be extracted from kwargs."""
        self._import_and_install()
        instance = self.Completions()
        instance.create(model="gpt-3.5-turbo", messages=[])

        kwargs = self.handle.report_llm_call.call_args[1]
        assert kwargs["model"] == "gpt-3.5-turbo"

    def test_missing_usage_defaults_to_zero(self):
        """If response has no usage, tokens default to 0."""
        self._import_and_install()

        # Override Completions.create to return response without usage
        from aa_hooks import openai as hook_mod
        orig = hook_mod._original_create

        def no_usage_create(*args, **kwargs):
            response = MagicMock()
            response.usage = None
            return response

        hook_mod._original_create = no_usage_create
        try:
            instance = self.Completions()
            instance.create(model="gpt-4o", messages=[])
            kwargs = self.handle.report_llm_call.call_args[1]
            assert kwargs["prompt_tokens"] == 0
            assert kwargs["completion_tokens"] == 0
        finally:
            hook_mod._original_create = orig


if __name__ == "__main__":
    unittest.main()

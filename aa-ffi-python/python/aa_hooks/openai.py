"""OpenAI adapter — monkey-patches ``openai`` to intercept chat completions.

Called by the Rust hook registry when ``openai`` is detected in
``sys.modules``.  Wraps ``openai.resources.chat.completions.Completions.create``
so that every chat completion call is reported as an ``LlmCallDetail`` audit
event through the ``AssemblyHandle`` command channel.

Safety guarantees
-----------------
* The original return value is **never modified** — the response object
  passes through untouched.
* Framework exceptions propagate unchanged — the wrapper re-raises
  immediately without swallowing.
* Capture or reporting failures are caught and silently dropped so that
  a broken hook can never break the user's application.
"""

from __future__ import annotations

import logging
import time
import warnings
from typing import Any

logger = logging.getLogger("aa_hooks.openai")

# Sentinel used to detect whether the hook has already been installed.
_original_create: Any | None = None
_original_async_create: Any | None = None


def install(handle: Any) -> None:
    """Install the OpenAI chat-completions monkey-patch.

    Parameters
    ----------
    handle:
        An ``AssemblyHandle`` instance (PyO3 class) that exposes
        ``report_llm_call(model, prompt_tokens, completion_tokens,
        latency_ms, provider)``.
    """
    global _original_create, _original_async_create

    try:
        import openai  # noqa: F811
        from openai.resources.chat.completions import Completions
    except ImportError as exc:
        warnings.warn(
            f"aa_hooks.openai: openai package not importable, skipping hook: {exc}",
            stacklevel=2,
        )
        return

    # Guard against double-install.
    if _original_create is not None:
        logger.debug("openai hook already installed, skipping")
        return

    _original_create = Completions.create

    def _wrapped_create(self: Any, *args: Any, **kwargs: Any) -> Any:
        model = kwargs.get("model", "unknown")
        start = time.monotonic()

        # Call the original — never swallow its exceptions.
        response = _original_create(self, *args, **kwargs)

        # Capture metadata and report (best-effort).
        try:
            latency_ms = int((time.monotonic() - start) * 1000)
            usage = getattr(response, "usage", None)
            prompt_tokens = getattr(usage, "prompt_tokens", 0) or 0
            completion_tokens = getattr(usage, "completion_tokens", 0) or 0

            handle.report_llm_call(
                model=str(model),
                prompt_tokens=int(prompt_tokens),
                completion_tokens=int(completion_tokens),
                latency_ms=latency_ms,
                provider="openai",
            )
        except Exception:
            # Never let reporting failures break the user's code.
            logger.debug("openai hook: failed to report llm call", exc_info=True)

        return response

    Completions.create = _wrapped_create  # type: ignore[assignment]

    # Also patch the async variant if available.
    try:
        from openai.resources.chat.completions import AsyncCompletions

        _original_async_create = AsyncCompletions.create

        async def _wrapped_async_create(self: Any, *args: Any, **kwargs: Any) -> Any:
            model = kwargs.get("model", "unknown")
            start = time.monotonic()

            response = await _original_async_create(self, *args, **kwargs)

            try:
                latency_ms = int((time.monotonic() - start) * 1000)
                usage = getattr(response, "usage", None)
                prompt_tokens = getattr(usage, "prompt_tokens", 0) or 0
                completion_tokens = getattr(usage, "completion_tokens", 0) or 0

                handle.report_llm_call(
                    model=str(model),
                    prompt_tokens=int(prompt_tokens),
                    completion_tokens=int(completion_tokens),
                    latency_ms=latency_ms,
                    provider="openai",
                )
            except Exception:
                logger.debug("openai hook: failed to report async llm call", exc_info=True)

            return response

        AsyncCompletions.create = _wrapped_async_create  # type: ignore[assignment]
    except (ImportError, AttributeError):
        # Async completions not available in this version — skip.
        pass

    logger.info("openai hook installed")


def uninstall() -> None:
    """Restore the original ``create`` methods.  Used by tests."""
    global _original_create, _original_async_create

    if _original_create is not None:
        try:
            from openai.resources.chat.completions import Completions

            Completions.create = _original_create  # type: ignore[assignment]
        except ImportError:
            pass
        _original_create = None

    if _original_async_create is not None:
        try:
            from openai.resources.chat.completions import AsyncCompletions

            AsyncCompletions.create = _original_async_create  # type: ignore[assignment]
        except ImportError:
            pass
        _original_async_create = None

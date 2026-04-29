"""Agent Assembly framework hook modules.

Each submodule exposes an ``install(handle)`` function that monkey-patches
a specific AI framework to intercept LLM calls and report them as audit
events through the ``AssemblyHandle`` command channel.

Submodules are imported lazily by the Rust hook registry
(``aa-ffi-python/src/hooks.rs``) — only the modules corresponding to
detected frameworks are loaded.
"""

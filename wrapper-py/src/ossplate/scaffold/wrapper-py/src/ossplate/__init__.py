from __future__ import annotations

from importlib import import_module

__all__ = ["cli", "get_binary_path", "get_packaged_binary_path", "main"]


def __getattr__(name: str):
    if name in __all__:
        module = import_module(f"{__name__}.cli")
        return getattr(module, name)
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")

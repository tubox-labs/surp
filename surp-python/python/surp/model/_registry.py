from __future__ import annotations

import inspect
import sys
import threading
from typing import Any


_lock = threading.RLock()
__surp_registry__: dict[str, type] = {}


def register(cls: type) -> type:
    r"""register(cls) -> type

    Register a model class by both RFC type name and Python class name.
    """
    name = getattr(cls, "__rfc_type__", cls.__name__)
    with _lock:
        __surp_registry__[name] = cls
        __surp_registry__[cls.__name__] = cls
    return cls


def get(name: str) -> type | None:
    r"""get(name) -> type | None

    Look up a registered model class by name.
    """
    return __surp_registry__.get(name)


def all_models() -> dict[str, type]:
    r"""all_models() -> dict[str, type]

    Return a snapshot of all registered model names and classes.
    """
    return dict(__surp_registry__)


def register_module(module: str | Any) -> None:
    r"""register_module(module) -> None

    Register every ``SurpModel`` subclass found in a module.

    Args:
        module (str or Any): Imported module object, or a module name present
          in ``sys.modules``.
    """
    if isinstance(module, str):
        module_obj = sys.modules[module]
    else:
        module_obj = module
    from ._base import SurpModel

    for _name, obj in inspect.getmembers(module_obj, inspect.isclass):
        if issubclass(obj, SurpModel) and obj is not SurpModel:
            register(obj)

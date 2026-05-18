from __future__ import annotations

import sys
import types

import pytest

from surp.model import Field, SurpModel, SurpModelDefinitionError
from surp.model.types import Int64, Str


def test_valid_model_fields_are_collected():
    class User(SurpModel):
        name: Str = Field(required=True, doc="Display name")
        age: Int64 = Field(required=False, default=0)

    assert list(User.__surp_fields__) == ["name", "age"]
    assert User.__surp_fields__["name"].required is True
    assert User.__surp_fields__["age"].default == 0


def test_python314_lazy_annotations_are_collected_without_future_import():
    module = types.ModuleType("_surp_lazy_annotation_test")
    sys.modules[module.__name__] = module
    try:
        exec(
            "\n".join(
                [
                    "from surp.model import Field, SurpModel",
                    "from surp.model.types import Bool, Int64, SeqOf, Str",
                    "class User(SurpModel):",
                    "    name: Str = Field(required=True)",
                    "    age: Int64 = Field(required=True)",
                    "    active: Bool = Field(required=True)",
                    "    tags: SeqOf[Str] = Field(required=False, default_factory=list)",
                ]
            ),
            module.__dict__,
        )
        User = module.User
        assert list(User.__surp_fields__) == ["name", "age", "active", "tags"]
        assert User(name="Alice", age=30, active=True).tags == []
    finally:
        sys.modules.pop(module.__name__, None)


def test_bare_python_annotation_is_rejected():
    with pytest.raises(SurpModelDefinitionError):

        class Bad(SurpModel):
            name: str = Field(required=True)


def test_annotation_without_field_is_rejected():
    with pytest.raises(SurpModelDefinitionError):

        class Bad(SurpModel):
            name: Str


def test_bare_default_is_rejected():
    with pytest.raises(SurpModelDefinitionError):

        class Bad(SurpModel):
            name: Str = "Alice"

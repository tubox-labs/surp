from __future__ import annotations

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

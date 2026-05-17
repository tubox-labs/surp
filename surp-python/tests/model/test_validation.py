from __future__ import annotations

import pytest

from surp.model import Field, SurpModel, SurpValidationError
from surp.model.types import Bool, Int64, SeqOf, Str, Tensor, TensorDType


class User(SurpModel):
    name: Str = Field(required=True)
    age: Int64 = Field(required=True)
    active: Bool = Field(required=True)
    tags: SeqOf[Str] = Field(required=False, default_factory=list)


def test_missing_required_field_raises():
    with pytest.raises(SurpValidationError) as exc:
        User(name="Alice", age=30)
    assert exc.value.errors[0].field_path == "active"


def test_collect_errors_returns_all_errors():
    user = User(name=12, age="old", active="yes", _validate=False)
    errors = user.collect_errors()
    assert [error.field_path for error in errors] == ["name", "age", "active"]


def test_tensor_shape_validation():
    class Embedding(SurpModel):
        vector: Tensor[TensorDType.F32, (3,)] = Field(required=True)

    assert Embedding(vector=[1.0, 2.0, 3.0]).vector == [1.0, 2.0, 3.0]
    with pytest.raises(SurpValidationError):
        Embedding(vector=[1.0, 2.0])

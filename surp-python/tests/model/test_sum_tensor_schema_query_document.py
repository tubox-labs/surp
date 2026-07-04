from __future__ import annotations

import pytest
from surp import rfc001

from surp.model import Field, SurpDocument, SurpModel, SurpQueryError, SurpVariant, annotation
from surp.model.types import F64, Str, SumOf, Tensor, TensorDType, Variant


class Shape(SurpModel):
    kind: SumOf[
        Variant["Circle", ("radius", F64)],
        Variant["Rectangle", ("width", F64), ("height", F64)],
        Variant["Point"],
    ] = Field(required=True)


class Embedding(SurpModel):
    label: Str = Field(required=True)
    vector: Tensor[TensorDType.F32, (3,)] = Field(required=True)


def test_sum_roundtrip():
    shape = Shape(kind=SurpVariant("Circle", {"radius": 2.5}))
    restored = Shape.from_cbf(shape.to_cbf())
    assert restored == shape


def test_tensor_roundtrip():
    embedding = Embedding(label="a", vector=[1.0, 2.0, 3.0])
    restored = Embedding.from_ctn(embedding.to_ctn())
    assert restored == embedding


def test_schema_ctn_is_parseable():
    rfc001.parse_ctn(Embedding.schema_ctn())
    assert Embedding.schema_json()["properties"]["vector"]["x-surp-type"].startswith("tensor")


def test_query_one_returns_scalar_and_raises_on_miss():
    embedding = Embedding(label="a", vector=[1.0, 2.0, 3.0])
    assert embedding.query_one(".label") == "a"
    with pytest.raises(SurpQueryError):
        embedding.query_one(".missing")


def test_query_does_not_mask_programming_errors_as_query_errors():
    # `surp.model._query.query` used to catch bare `Exception`, so a genuine
    # bug (e.g. an AttributeError from a broken `to_cbf()`) was silently
    # relabeled as SurpQueryError instead of propagating as-is.
    from surp.model._query import query

    class Broken:
        def to_cbf(self):
            raise AttributeError("boom: not a real model")

    with pytest.raises(AttributeError, match="boom"):
        query(Broken(), ".label")


@annotation("surp", "1.0")
class UserDocument(SurpDocument):
    __root__ = "user"

    user: Embedding = Field(required=True, binding="user")


def test_document_annotations_and_binding_roundtrip():
    doc = UserDocument(user=Embedding(label="a", vector=[1.0, 2.0, 3.0]))
    ctn = doc.to_ctn()
    assert "@surp" in ctn
    restored = UserDocument.from_ctn(ctn)
    assert restored == doc

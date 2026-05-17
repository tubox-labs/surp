from __future__ import annotations

import pytest

from surp.model import Field, SurpDecodeModelError, SurpModel
from surp.model.types import Int64, Nullable, OneOf, Str, Tagged


class StrictUser(SurpModel):
    name: Str = Field(required=True)


class LooseUser(SurpModel):
    __rfc_type__ = "StrictUser"
    __strict__ = False

    name: Str = Field(required=True)


def test_strict_rejects_extra_fields_on_decode():
    with pytest.raises(SurpDecodeModelError):
        StrictUser.from_ctn('StrictUser\n  name = "Alice"\n  extra = "x"')


def test_non_strict_allows_extra_fields_on_decode():
    user = LooseUser.from_ctn('StrictUser\n  name = "Alice"\n  extra = "x"')
    assert user.name == "Alice"


def test_from_dict_obeys_strict_unknown_field_policy():
    with pytest.raises(SurpDecodeModelError):
        StrictUser.from_dict({"name": "Alice", "extra": "x"})

    user = LooseUser.from_dict({"name": "Alice", "extra": "x"})
    assert user.name == "Alice"


class Record(SurpModel):
    uid: Tagged["uid", Str] = Field(required=True)
    label: Nullable[Str] = Field(required=False)
    payload: OneOf[Str, Int64] = Field(required=True)


def test_tagged_nullable_and_oneof_roundtrip():
    record = Record(uid="abc", label=None, payload=7)
    restored = Record.from_cbf(record.to_cbf())
    assert restored == record

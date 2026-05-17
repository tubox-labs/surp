from __future__ import annotations

import pytest

from surp.model import Field, SurpModel, SurpSymbolEnum
from surp.model.types import Str


class Role(SurpSymbolEnum):
    ADMIN = "Admin"
    VIEWER = "Viewer"


class User(SurpModel):
    name: Str = Field(required=True)
    role: Role = Field(required=True)


def test_symbol_enum_roundtrip():
    user = User(name="Alice", role=Role.ADMIN)
    restored = User.from_cbf(user.to_cbf())
    assert restored.role is Role.ADMIN


def test_symbol_enum_rejects_unknown_when_strict():
    with pytest.raises(Exception):
        User.from_ctn('User\n  name = "Alice"\n  role = \'Owner')


def test_symbol_enum_allows_unknown_when_not_strict():
    class LooseUser(SurpModel):
        __rfc_type__ = "User"
        __strict__ = False

        name: Str = Field(required=True)
        role: Role = Field(required=True)

    restored = LooseUser.from_ctn('User\n  name = "Alice"\n  role = \'Owner')
    assert restored.role == "Owner"

from __future__ import annotations

from surp.model import Field, SurpModel
from surp.model.types import Bool, Int64, MapOf, SeqOf, Str


class Address(SurpModel):
    street: Str = Field(required=True)
    city: Str = Field(required=True)


class User(SurpModel):
    name: Str = Field(required=True)
    age: Int64 = Field(required=False, default=0)
    active: Bool = Field(required=True)
    tags: SeqOf[Str] = Field(required=False, default_factory=list)
    settings: MapOf[Str, Str] = Field(required=False, default_factory=dict)
    address: Address = Field(required=True)


def test_ctn_roundtrip():
    user = User(
        name="Alice",
        active=True,
        tags=["admin", "ops"],
        settings={"theme": "dark"},
        address=Address(street="1 Main", city="Paris"),
    )
    restored = User.from_ctn(user.to_ctn())
    assert restored == user


def test_cbf_roundtrip():
    user = User(name="Alice", active=True, address=Address(street="1 Main", city="Paris"))
    restored = User.from_cbf(user.to_cbf(alignment=4))
    assert restored == user


def test_required_empty_map_roundtrip_is_canonical():
    class RequiredSettings(SurpModel):
        settings: MapOf[Str, Str] = Field(required=True)

    instance = RequiredSettings(settings={})
    ctn = instance.to_ctn()
    assert "map<any, any> []" in ctn
    assert RequiredSettings.from_ctn(ctn) == instance


def test_to_surp_uses_v1_convenience_path():
    user = User(name="Alice", active=True, address=Address(street="1 Main", city="Paris"))
    assert isinstance(user.to_surp(), bytes)

from __future__ import annotations

from surp.model import Field, SurpModel, generate_model_stubs
from surp.model.types import Bool, Int64, SeqOf, Str


class StubUser(SurpModel):
    name: Str = Field(required=True)
    age: Int64 = Field(required=False, default=0)
    active: Bool = Field(required=True)
    tags: SeqOf[Str] = Field(required=False, default_factory=list)


def test_generate_model_stubs_exposes_typed_keyword_init():
    stub = generate_model_stubs(StubUser)
    assert "class StubUser(SurpModel):" in stub
    assert (
        'def __init__(self, *, name: str, age: int = ..., active: bool, tags: list[str] = ...) -> None: ...'
        in stub
    )

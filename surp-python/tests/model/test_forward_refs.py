from __future__ import annotations

from surp.model import Field, SurpModel
from surp.model.types import SeqOf, Str


def test_self_referential_sequence_roundtrip():
    class Post(SurpModel):
        title: Str = Field(required=True)
        replies: SeqOf["Post"] = Field(required=False, default_factory=list)

    post = Post(title="root", replies=[Post(title="child")])
    restored = Post.from_ctn(post.to_ctn())
    assert restored == post

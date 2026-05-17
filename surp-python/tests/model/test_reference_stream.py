from __future__ import annotations

from surp.model import Field, SurpModel, SurpStream
from surp.model.types import RefOf, Str, StreamOf


class RefRecord(SurpModel):
    value: RefOf[Str] = Field(required=True)


class StreamRecord(SurpModel):
    events: StreamOf[Str] = Field(required=True)


def test_reference_field_roundtrip():
    record = RefRecord(value="abc")
    ctn = record.to_ctn()
    assert 'ref "abc"' in ctn
    assert RefRecord.from_cbf(record.to_cbf()) == record


def test_stream_field_roundtrip():
    record = StreamRecord(events=SurpStream({"encoding": "utf-8"}))
    ctn = record.to_ctn()
    assert 'stream<str>' in ctn
    assert '@encoding "utf-8"' in ctn
    assert StreamRecord.from_ctn(ctn) == record

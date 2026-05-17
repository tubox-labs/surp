"""End-to-end v1 Surp example for the native Python package."""

from __future__ import annotations

from io import BytesIO

import surp


PAYLOAD = {
    "id": 1001,
    "name": "Alice",
    "active": True,
    "tags": ["admin", "ops"],
    "avatar": b"\x01\x02\x03",
    "settings": {"theme": "dark", "region": "us"},
}


def main() -> None:
    data = surp.dumps(PAYLOAD, dedup=True, sort_keys=True)
    decoded = surp.loads(data)
    assert decoded == PAYLOAD

    view = surp.loads_value(data)
    assert isinstance(view, surp.SurpValue)
    assert view.kind == "object"
    assert view["name"].value == "Alice"
    assert view["tags"][1].value == "ops"
    assert view.as_python() == PAYLOAD

    text = surp.pretty_print(decoded, indent=2)
    reparsed = surp.parse_text(text)
    assert reparsed == PAYLOAD

    file_obj = BytesIO()
    surp.dump(PAYLOAD, file_obj, sort_keys=True)
    file_obj.seek(0)
    assert surp.load(file_obj) == PAYLOAD

    encoder = surp.Encoder(sort_keys=True)
    encoder.enable_dedup()
    encoder.set_compression("none")
    encoder.encode({"kind": "event", "name": "created"})
    encoder.encode({"kind": "event", "name": "updated"})
    stream = encoder.finish()

    decoder = surp.SurpDecoder(stream)
    assert decoder.decode_all() == [
        {"kind": "event", "name": "created"},
        {"kind": "event", "name": "updated"},
    ]

    print(f"v1 payload round-tripped in {len(data)} bytes")
    print(text)


if __name__ == "__main__":
    main()

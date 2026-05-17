"""Regression tests for the native ``surp`` Python package."""

from __future__ import annotations

from io import BytesIO

import pytest
import surp
from surp import rfc001


class TestEncodeDecode:
    def test_roundtrip_primitives(self):
        cases = [
            None,
            True,
            False,
            0,
            42,
            -42,
            2**40,
            3.15,
            "",
            "hello world",
            b"\x00\x01\x02\xff",
        ]
        for obj in cases:
            assert surp.decode(surp.encode(obj)) == obj

    def test_roundtrip_nested_mixed_payload(self):
        obj = {
            "users": [
                {"name": "Alice", "active": True, "roles": ["admin", "ops"]},
                {"name": "Bob", "active": False, "roles": []},
            ],
            "count": 2,
            "blob": b"\xab\xcd",
            "meta": {"empty_dict": {}, "empty_list": []},
        }
        assert surp.loads(surp.dumps(obj, dedup=True, sort_keys=True)) == obj

    def test_tuple_encodes_as_array(self):
        assert surp.decode(surp.encode((1, "two", True))) == [1, "two", True]

    def test_file_like_roundtrip(self):
        obj = {"name": "Alice", "age": 30}
        fp = BytesIO()
        surp.dump(obj, fp)
        fp.seek(0)
        assert surp.load(fp) == obj

    def test_encode_to_file_roundtrip(self, tmp_path):
        path = tmp_path / "data.surp"
        obj = {"name": "Alice", "age": 30}
        surp.encode_to_file(obj, path)
        assert surp.decode_from_file(path) == obj

    def test_unsupported_type_raises_native_type_error(self):
        with pytest.raises(surp.SurpTypeError):
            surp.encode(object())

    def test_invalid_data_raises_decode_error(self):
        with pytest.raises(surp.SurpDecodeError):
            surp.decode(b"not a surp file")


class TestEncoderDecoderClasses:
    def test_incremental_encoder_decoder(self):
        enc = surp.Encoder(sort_keys=True)
        enc.enable_dedup()
        enc.set_compression("none")
        enc.encode({"key": "value"})
        data = enc.finish()

        dec = surp.SurpDecoder(data)
        assert dec.decode_all() == [{"key": "value"}]

    def test_encoder_finish_is_one_shot(self):
        enc = surp.Encoder()
        enc.encode(42)
        enc.finish()
        with pytest.raises(surp.SurpEncodeError):
            enc.finish()
        with pytest.raises(surp.SurpEncodeError):
            enc.encode(99)

    def test_decoder_is_one_shot(self):
        dec = surp.SurpDecoder(surp.encode(42))
        assert dec.decode_all() == [42]
        with pytest.raises(surp.SurpDecodeError):
            dec.decode_all()

    def test_invalid_compression_raises_value_error(self):
        enc = surp.Encoder()
        with pytest.raises(ValueError):
            enc.set_compression("invalid")


RFC_COMPLEX_CTN = """
@surp v1
@encoding cbf

let alice = User
  id = uid"550e8400-e29b-41d4-a716-446655440000"
  name = "Alice"
  role = 'Admin
  tags = ["admin", "ops"]
  settings = map<str, str> ["theme" => "dark", 'region => "us"]
  matrix = tensor<f32>[2, 2]
    [1.0f32, 2.0f32]
    [3.0f32, 4.0f32]

&alice
"""


class TestRfc001Ctn:
    def test_parse_ctn_preserves_document_shape(self):
        doc = rfc001.parse_ctn(RFC_COMPLEX_CTN)
        assert doc["annotations"][0]["name"] == "surp"
        assert doc["bindings"][0]["name"] == "alice"
        assert doc["root"]["kind"] == "reference"
        assert doc["root"]["reference_kind"] == "binding"

        user = doc["bindings"][0]["value"]
        assert user["kind"] == "product"
        assert user["type_name"] == "User"
        assert [field["name"] for field in user["fields"]] == [
            "id",
            "name",
            "role",
            "tags",
            "settings",
            "matrix",
        ]

    def test_normalize_ctn_roundtrips_parse(self):
        normalized = rfc001.normalize_ctn(RFC_COMPLEX_CTN)
        reparsed = rfc001.parse_ctn(normalized)
        assert reparsed["bindings"][0]["name"] == "alice"
        assert "User" in normalized
        assert "tensor<f32>[2, 2]" in normalized

    def test_invalid_ctn_raises_rfc_error(self):
        with pytest.raises(surp.SurpRfcError):
            rfc001.parse_ctn("let 123bad = true")


class TestRfc001Cbf:
    def test_compile_and_decode_cbf_exposes_header_symbols_and_ctn(self):
        data = rfc001.compile_ctn(RFC_COMPLEX_CTN, alignment=4)
        assert data[:4] == rfc001.CBF_MAGIC
        assert rfc001.CBF_HEADER_SIZE == 32

        decoded = rfc001.decode_cbf(data)
        assert decoded["header"]["magic"] == "SURP"
        assert decoded["header"]["cbf_version"] == 1
        assert decoded["header"]["ctn_version"] == 1
        assert decoded["header"]["alignment"] == 4
        assert decoded["header"]["has_symtab"] is True
        assert "name" in decoded["symbols"]
        assert "role" in decoded["symbols"]
        assert decoded["document"]["root"]["kind"] == "product"
        assert "Alice" in decoded["ctn"]
        assert rfc001.cbf_to_ctn(data) == decoded["ctn"]

    def test_compile_without_symbol_table_rejects_symbol_values(self):
        with pytest.raises(surp.SurpRfcError):
            rfc001.compile_ctn(RFC_COMPLEX_CTN, with_symtab=False)

    def test_compile_without_symbol_table_allows_plain_scalars(self):
        data = rfc001.compile_ctn('Plain\n  name = "Alice"', with_symtab=False)
        decoded = rfc001.decode_cbf(data)
        assert decoded["header"]["has_symtab"] is False
        assert decoded["symbols"] == []

    def test_checksum_corruption_is_rejected(self):
        data = bytearray(rfc001.compile_ctn('User\n  name = "Alice"'))
        data[-1] ^= 0xFF
        with pytest.raises(surp.SurpRfcError):
            rfc001.decode_cbf(bytes(data))

    def test_cyclic_binding_reference_is_rejected(self):
        with pytest.raises(surp.SurpRfcError):
            rfc001.compile_ctn("let a = &b\nlet b = &a\n&a")


class TestRfc001Cql:
    def test_query_ctn_field_sequence_map_and_tensor(self):
        name = rfc001.query_ctn(RFC_COMPLEX_CTN, ".name")
        assert name[0]["type"] == "str"
        assert name[0]["value"] == "Alice"

        last_tag = rfc001.query_ctn(RFC_COMPLEX_CTN, ".tags[-1]", as_ctn=True)
        assert last_tag == ['"ops"']

        theme = rfc001.query_ctn(RFC_COMPLEX_CTN, ".settings['theme]")
        assert theme[0]["value"] == "dark"

        matrix = rfc001.query_ctn(RFC_COMPLEX_CTN, ".matrix")
        assert matrix[0]["kind"] == "tensor"
        assert matrix[0]["shape"] == [2, 2]
        assert matrix[0]["data"]["kind"] == "dense_f64"
        assert matrix[0]["data"]["values"] == [1.0, 2.0, 3.0, 4.0]

    def test_query_cbf_matches_query_ctn(self):
        data = rfc001.compile_ctn(RFC_COMPLEX_CTN)
        assert rfc001.query_cbf(data, ".name", as_ctn=True) == ['"Alice"']
        assert rfc001.query_cbf(data, ".tags[]", as_ctn=True) == ['"admin"', '"ops"']

    def test_invalid_cql_raises_rfc_error(self):
        data = rfc001.compile_ctn('User\n  name = "Alice"')
        with pytest.raises(surp.SurpRfcError):
            rfc001.query_cbf(data, "name")

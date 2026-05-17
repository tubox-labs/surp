"""End-to-end RFC-001 CTN / CBF / CQL example for the native surp package."""

from __future__ import annotations

import surp


CTN = """
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

PLAIN_CTN = """
User
  name = "Alice"
  tags = ["admin", "ops"]
  settings = map<str, str> ["theme" => "dark"]
"""


def main() -> None:
    parsed = surp.rfc001.parse_ctn(CTN)
    assert parsed["bindings"][0]["name"] == "alice"

    normalized = surp.rfc001.normalize_ctn(CTN)
    assert "tensor<f32>[2, 2]" in normalized

    cbf = surp.rfc001.compile_ctn(CTN, alignment=4)
    decoded = surp.rfc001.decode_cbf(cbf)
    assert decoded["header"]["magic"] == "SURP"
    assert decoded["header"]["alignment"] == 4
    assert decoded["header"]["has_symtab"] is True

    assert surp.rfc001.cbf_to_ctn(cbf) == decoded["ctn"]
    assert surp.rfc001.query_cbf(cbf, ".name", as_ctn=True) == ['"Alice"']
    assert surp.rfc001.query_cbf(cbf, ".tags[]", as_ctn=True) == ['"admin"', '"ops"']

    theme = surp.rfc001.query_ctn(CTN, ".settings['theme]")
    assert theme[0]["kind"] == "scalar"
    assert theme[0]["value"] == "dark"

    plain = surp.rfc001.compile_ctn(PLAIN_CTN, with_symtab=False)
    assert surp.rfc001.decode_cbf(plain)["header"]["has_symtab"] is False

    print(decoded["header"])
    print(surp.rfc001.query_cbf(cbf, ".matrix", as_ctn=True))


if __name__ == "__main__":
    main()

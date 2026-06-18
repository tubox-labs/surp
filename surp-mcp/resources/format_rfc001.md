# Surp RFC-001 Format

RFC-001 is implemented beside v1 and uses separate data types.

- CTN is the indentation-oriented text format.
- CBF is the binary `.crb` segment-tree format with `SURP` magic, a 32-byte header, optional symbol table, root offset, and CRC64 trailer.
- CQL support is the baseline path subset: `.field`, `[]`, `[index]`, negative indexes, `['symbol]`, and `["string"]`.
- CTN binding references are resolved when compiling to CBF.

Do not treat RFC-001 CBF as v1 `.surp`; use the `surp_rfc001_*` tools for CTN/CBF/CQL workflows.

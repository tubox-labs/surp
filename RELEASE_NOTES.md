# Surp v1.1.3 Release Notes

Surp v1.1.3 completes the project rename from `crous` to `surp`, removes the
legacy binary header from the standard format, and publishes the final
legacy `crous` Python package metadata for migration.

## Highlights

- Project identity is now `surp` across Rust crates, Python packages, CLI names,
  FFI symbols, workflows, benchmarks, and documentation.
- The standard binary format no longer includes the legacy binary header. Encoded
  payloads begin directly with the block stream header.
- Python package version metadata reports `1.1.3` for the pure-Python package
  and the PyO3 native extension.
- The legacy `crous` Python package at version `1.1.3` emits:

  ```py
  # setup message
  warnings.warn(
      "Package 'crous' has moved to 'surp'. Install with: pip install surp",
      DeprecationWarning
  )
  ```

## Migration

Install the renamed package:

```sh
pip install surp
```

Update imports:

```py
import surp
```

Update repository remotes:

```sh
git remote set-url origin https://github.com/tubox-labs/surp.git
```

## Compatibility

The `crous` package should only be released at `1.1.3` as a deprecation shim.
New consumers should install `surp` directly.

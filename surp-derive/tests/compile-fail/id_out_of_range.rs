// An `id` literal that doesn't fit in a `u64` must produce a clean compile
// error instead of panicking inside the proc-macro (which previously
// happened via a bare `.unwrap()` on `base10_parse`).
use surp_derive::Surp;

#[derive(Surp)]
struct OutOfRange {
    #[surp(id = 99999999999999999999)]
    name: String,
}

fn main() {}

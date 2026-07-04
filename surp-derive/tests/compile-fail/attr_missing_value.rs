// `#[surp(id)]` with no `= value` must be a compile error, not silently
// ignored in favor of the hash-based fallback id.
use surp_derive::Surp;

#[derive(Surp)]
struct MissingValue {
    #[surp(id)]
    name: String,
}

fn main() {}

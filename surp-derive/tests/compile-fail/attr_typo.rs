// A typo'd attribute key (`idd` instead of `id`) must be a compile error,
// not silently ignored in favor of the hash-based fallback id.
use surp_derive::Surp;

#[derive(Surp)]
struct Typo {
    #[surp(idd = 1)]
    name: String,
}

fn main() {}

// Two fields explicitly sharing the same `#[surp(id = N)]` must be a
// compile error: they would silently collide on the same wire key.
use surp_derive::Surp;

#[derive(Surp)]
struct DuplicateId {
    #[surp(id = 1)]
    first: String,
    #[surp(id = 1)]
    second: String,
}

fn main() {}

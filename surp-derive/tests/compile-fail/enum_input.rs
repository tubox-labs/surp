// `#[derive(Surp)]` only supports structs; an enum must produce a clean
// compile error instead of a macro panic.
use surp_derive::Surp;

#[derive(Surp)]
enum NotAStruct {
    A,
    B,
}

fn main() {}

// `#[derive(Surp)]` only supports structs with named fields; a tuple struct
// must produce a clean compile error instead of a macro panic.
use surp_derive::Surp;

#[derive(Surp)]
struct TupleStruct(#[surp(id = 1)] String);

fn main() {}

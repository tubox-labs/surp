// `derive_surp_schema` resolves field ids independently of `derive_surp` (it
// is a separate proc-macro invocation), so it needs its own duplicate-id
// check exercised here with `#[derive(SurpSchema)]` alone.
use surp_derive::SurpSchema;

#[derive(SurpSchema)]
struct DuplicateIdSchema {
    #[surp(id = 5)]
    first: String,
    #[surp(id = 5)]
    second: String,
}

fn main() {}

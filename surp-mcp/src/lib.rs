//! MCP stdio server for Surp.
//!
//! The crate intentionally implements a small JSON-RPC stdio transport instead
//! of shelling out to `surp-cli`, so tools call `surp-core` directly.

mod ops;
mod protocol;
mod registry;
mod security;

pub use protocol::{handle_message, run_stdio};

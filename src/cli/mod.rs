pub mod core_dispatcher;
pub mod entry;
pub mod entry_subcommands;
pub mod presenter;
pub mod presenter_json;
pub mod presenter_sink;

// ---------------------------------------------------------------------------
// CLI exit codes
// ---------------------------------------------------------------------------

pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_GENERAL_FAILURE: i32 = 1;
pub const EXIT_VALIDATION_FAILURE: i32 = 2;
pub const EXIT_PARTIAL_SUCCESS: i32 = 3;

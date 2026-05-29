pub(crate) mod core_dispatcher;
pub(crate) mod entry;
pub(crate) mod presenter;

// ---------------------------------------------------------------------------
// CLI exit codes
// ---------------------------------------------------------------------------

pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_GENERAL_FAILURE: i32 = 1;
pub const EXIT_VALIDATION_FAILURE: i32 = 2;
pub const EXIT_PARTIAL_SUCCESS: i32 = 3;

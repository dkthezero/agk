/// State for the F3 profile editor modal.
pub struct EditProfileState {
    /// Profile name being edited (read-only).
    pub profile_name: String,
    /// Active field: 0 = skills, 1 = mcps, 2 = permission_mode.
    pub field_index: usize,
    /// Cursor / selected item within the active field.
    pub selected: usize,
    /// Available skill names.
    pub skills: Vec<String>,
    /// Checkbox state for skills (parallel with `skills`).
    pub skills_checked: Vec<bool>,
    /// Available MCP names.
    pub mcps: Vec<String>,
    /// Checkbox state for MCPs (parallel with `mcps`).
    pub mcps_checked: Vec<bool>,
    /// Available permission modes.
    pub permission_modes: Vec<String>,
    /// Selected permission mode index.
    pub permission_index: usize,
    /// Estimated token count for the profile (advisory only).
    pub estimated_tokens: usize,
}

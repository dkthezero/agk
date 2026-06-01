use crate::domain::asset::AssetKind;

/// Classification of a feature-set tab, used by both the application bootstrap
/// layer (to decide rendering characteristics) and the TUI layer (to map it to
/// concrete UI behaviour).
///
/// This type intentionally lives in `app/` (not `tui/`) so that the composition
/// root (`app/bootstrap.rs`) can build tab metadata without violating the
/// hexagonal boundary rule that `app/` must not depend on `tui/`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TabKind {
    /// Skill- or instruction-like package assets.
    Asset,
    /// Vault source management.
    Vault,
    /// AI provider activation.
    Provider,
    /// MCP server list.
    Mcp,
    /// Telemetry / analytics (future).
    Analytics,
    /// Profile list / wizard.
    Profile,
}

impl TabKind {
    pub fn is_asset_like(self) -> bool {
        matches!(self, TabKind::Asset)
    }

    pub fn asset_label(self) -> Option<&'static str> {
        match self {
            TabKind::Asset => Some("Skills/Instructions"),
            _ => None,
        }
    }
}

/// Map a [`FeatureSetPort`] `kind_name` string to a canonical [`TabKind`].
pub fn tab_kind_for_feature_name(name: &str) -> TabKind {
    match name {
        "vault" => TabKind::Vault,
        "provider" => TabKind::Provider,
        "mcp" => TabKind::Mcp,
        "profile" => TabKind::Profile,
        "analytics" => TabKind::Analytics,
        // skill, instruction, and any future stub types render as generic
        // asset tabs.
        _ => TabKind::Asset,
    }
}

/// Given a scanned package's [`AssetKind`], return the [`TabKind`] that this
/// package belongs to.  Used when a tab kind must be derived from domain data
/// rather than feature-set metadata.
pub fn tab_kind_for_asset_kind(kind: &AssetKind) -> TabKind {
    match kind {
        AssetKind::Skill | AssetKind::Instruction => TabKind::Asset,
        AssetKind::McpServer => TabKind::Mcp,
        AssetKind::Profile => TabKind::Profile,
    }
}

# Provider Config Root Selection — Technical Design

## Architecture

Approach B: extended trait method + workspace config.

### ProviderPort trait additions

```rust
pub trait ProviderPort: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn install(
        &self,
        pkg: &ScannedPackage,
        scope: Scope,
        config: Option<&ConfigFile>,
    ) -> Result<()>;
    fn remove(
        &self,
        identity: &AssetIdentity,
        kind: &AssetKind,
        scope: Scope,
        config: Option<&ConfigFile>,
    ) -> Result<()>;
    fn install_path_for(&self, ...) -> Option<PathBuf> {
        None
    }

    /// NEW
    fn available_config_roots(&self) -> Vec<(String, String)> {
        // (folder_name, description)
        vec![]
    }
}
```

### ConfigFile additions

```rust
pub struct ConfigFile {
    // ... existing ...
    #[serde(default)]
    pub provider_roots: HashMap<String, String>,
}
```

### Provider behavior change

Each provider's `provider_root()` must now:
1. Check `config.provider_roots.get(self.id())`.
2. If found, use that folder name instead of hardcoded default.
3. Fall back to hardcoded default if not in config.

Example for OpenCode:
```rust
fn provider_root(&self, scope: &Scope, config: Option<&ConfigFile>) -> PathBuf {
    let folder = config
        .and_then(|c| c.provider_roots.get(self.id()))
        .map(|s| s.as_str())
        .unwrap_or(".opencode");
    match scope {
        Scope::Global => dirs_next::home_dir().unwrap().join(".config/opencode"),
        Scope::Workspace => self.workspace_root.join(folder),
    }
}
```

### TUI Modal State

New `ListMode` variant in `AppState`:
```rust
pub enum ListMode {
    // ... existing ...
    SelectProviderRoot {
        provider_id: String,
        options: Vec<(String, String)>, // (folder, description)
        selected: usize,
    },
}
```

### Rendering

The modal is drawn **after** the normal `draw()` call by:
1. Computing a centered `Rect` (≤60% width, 6-10 rows).
2. `frame.render_widget(Clear, area)` to erase background.
3. Rendering a `Block` with borders + title, containing a `List` of options with the current row highlighted in cyan.

### Key handling

When `list_mode == SelectProviderRoot`:
- `↑/↓` — cycle selection
- `Enter` — confirm, save to config, set provider active
- `Esc` — cancel, return to Normal mode without enabling provider

### Bootstrap wiring

Provider instances are constructed with `workspace_root` but **not** with config yet (config is loaded after providers are built). So `provider_root()` must accept the config lazily (passed at call time from actions).

### Testing strategy
- [x] Unit test: `OpenCodeProvider::provider_root` respects config override.
- [x] Unit test: `ConfigFile` round-trips `provider_roots` in TOML.
- [x] Unit test: TUI `SelectProviderRoot` state transitions correctly on Enter/Esc.
- [x] Integration: enable OpenCode in TUI, select `.agents`, verify skill installs to `.agents/skills/`.

## Module changes

```
src/
  app/ports.rs           — add available_config_roots
  domain/config.rs       — add provider_roots HashMap
  infra/provider/
    opencode.rs          — use config-driven root
    claude_code.rs       — use config-driven root, add .agents option
    gemini.rs            — use config-driven root, add .ai option
    ...                  — others default to single root
  tui/app.rs             — add SelectProviderRoot to ListMode
  tui/event.rs           — handle root selection keys
  tui/render.rs          — detect SelectProviderRoot, draw modal
  tui/widgets/modal.rs   — NEW: reusable centered modal renderer
```

*End of Technical Design.*

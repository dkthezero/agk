//! Per-entity detail-panel renderers (vault / provider / profile).
//!
//! Extracted from `detail.rs` to keep it under the 300-LOC ADR-001 §6.4
//! limit. Re-exported by `detail.rs` so callers in `tui/render/content.rs`
//! continue to use `crate::tui::widgets::detail::render_*_detail`.

use crate::app::snapshot::{ProfileEntry, ProviderEntry, VaultEntry};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_vault_detail(frame: &mut Frame, area: Rect, vault: Option<&VaultEntry>) {
    let block = Block::default().borders(Borders::ALL).title("Detail");
    let lines: Vec<Line> = match vault {
        None => vec![Line::from("  No vault selected")],
        Some(v) => {
            let label = |s: &str| Span::styled(s.to_string(), Style::default().fg(Color::Yellow));
            vec![
                Line::from(vec![label("Vault ID: "), Span::raw(v.id.clone())]),
                Line::from(vec![label("Type:     "), Span::raw(v.kind.clone())]),
                Line::from(vec![
                    label("Enabled:  "),
                    Span::raw(if v.enabled { "yes" } else { "no" }),
                ]),
                Line::from(Span::raw("")),
                Line::from(vec![label("Source:   "), Span::raw(v.source_path.clone())]),
                Line::from(Span::raw("")),
                Line::from(vec![
                    label("Skills:       "),
                    Span::raw(format!(
                        "{} installed / {} available",
                        v.installed_skills, v.available_skills
                    )),
                ]),
                Line::from(vec![
                    label("Instructions: "),
                    Span::raw(format!(
                        "{} installed / {} available",
                        v.installed_instructions, v.available_instructions
                    )),
                ]),
            ]
        }
    };
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

pub fn render_provider_detail(
    frame: &mut Frame,
    area: Rect,
    provider: Option<&ProviderEntry>,
    active_scope: crate::domain::scope::Scope,
) {
    let block = Block::default().borders(Borders::ALL).title("Detail");
    let lines: Vec<Line> = match provider {
        None => vec![Line::from("  No provider selected")],
        Some(p) => {
            let label = |s: &str| Span::styled(s.to_string(), Style::default().fg(Color::Yellow));
            let scope_paths = provider_scope_paths(&p.id, active_scope);
            let mut lines = vec![
                Line::from(vec![label("Provider:  "), Span::raw(p.name.clone())]),
                Line::from(vec![
                    label("Supported: "),
                    Span::raw("Agent Skills, Instructions"),
                ]),
                Line::from(Span::raw("")),
            ];
            for (label_text, path) in scope_paths {
                lines.push(Line::from(vec![
                    Span::styled(label_text, Style::default().fg(Color::Yellow)),
                    Span::raw(path),
                ]));
            }
            lines
        }
    };
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

/// Return scoped install/config paths for a provider id.
/// Returns a Vec of (label, path) for the detail panel.
fn provider_scope_paths(id: &str, scope: crate::domain::scope::Scope) -> Vec<(String, String)> {
    let is_global = matches!(scope, crate::domain::scope::Scope::Global);
    match id {
        "claude-code" => {
            let base = if is_global { "~/.claude" } else { ".claude" };
            vec![
                ("Skills:       ".into(), format!("{}/skills/<name>", base)),
                (
                    "Instructions: ".into(),
                    format!("{}/instructions/<name>", base),
                ),
                ("MCP Config:   ".into(), format!("{}/mcp.json", base)),
            ]
        }
        "github-copilot" => {
            let base = if is_global { "~/.copilot" } else { ".github" };
            vec![
                ("Skills:       ".into(), format!("{}/skills/<name>", base)),
                (
                    "Instructions: ".into(),
                    format!("{}/instructions/<name>", base),
                ),
                ("MCP Config:   ".into(), format!("{}/mcp-config.json", base)),
            ]
        }
        "gemini-cli" => {
            let base = if is_global { "~/.gemini" } else { ".gemini" };
            vec![
                ("Skills:       ".into(), format!("{}/skills/<name>", base)),
                (
                    "Instructions: ".into(),
                    format!("{}/instructions/<name>", base),
                ),
                ("MCP Config:   ".into(), format!("{}/settings.json", base)),
            ]
        }
        "opencode" => {
            let base = if is_global {
                "~/.config/opencode"
            } else {
                ".opencode"
            };
            let mcp = if is_global {
                "~/.config/opencode/opencode.json"
            } else {
                "opencode.json"
            };
            vec![
                ("Skills:       ".into(), format!("{}/skills/<name>", base)),
                (
                    "Instructions: ".into(),
                    format!("{}/instructions/<name>", base),
                ),
                ("MCP Config:   ".into(), mcp.into()),
            ]
        }
        "amp" => {
            let base = if is_global { "~/.amp" } else { ".amp" };
            let mcp = if is_global {
                "~/.config/amp/settings.json"
            } else {
                ".amp/settings.json"
            };
            vec![
                ("Skills:       ".into(), format!("{}/skills/<name>", base)),
                (
                    "Instructions: ".into(),
                    format!("{}/instructions/<name>", base),
                ),
                ("MCP Config:   ".into(), mcp.to_string()),
            ]
        }
        "letta" => {
            let base = if is_global { "~/.letta" } else { ".letta" };
            vec![
                ("Skills:       ".into(), format!("{}/skills/<name>", base)),
                (
                    "Instructions: ".into(),
                    format!("{}/instructions/<name>", base),
                ),
            ]
        }
        "snowflake" => {
            let base = if is_global { "~/.cortex" } else { ".cortex" };
            vec![
                ("Skills:       ".into(), format!("{}/skills/<name>", base)),
                (
                    "Instructions: ".into(),
                    format!("{}/instructions/<name>", base),
                ),
            ]
        }
        "firebender" => {
            let base = if is_global {
                "~/.firebender"
            } else {
                ".firebender"
            };
            vec![
                ("Skills:       ".into(), format!("{}/skills/<name>", base)),
                (
                    "Instructions: ".into(),
                    format!("{}/instructions/<name>", base),
                ),
            ]
        }
        _ => {
            let base = if is_global {
                "~/.config/<provider>"
            } else {
                ".<provider>"
            };
            vec![
                ("Skills:       ".into(), format!("{}/skills/<name>", base)),
                (
                    "Instructions: ".into(),
                    format!("{}/instructions/<name>", base),
                ),
            ]
        }
    }
}

pub fn render_profile_detail(frame: &mut Frame, area: Rect, profile: Option<&ProfileEntry>) {
    let block = Block::default().borders(Borders::ALL).title("Detail");
    let lines: Vec<Line> = match profile {
        None => vec![Line::from("  No profile selected")],
        Some(p) => {
            let label = |s: &str| Span::styled(s.to_string(), Style::default().fg(Color::Yellow));
            let mut lines = vec![
                Line::from(vec![label("Name:     "), Span::raw(p.name.clone())]),
                Line::from(vec![label("Provider: "), Span::raw(p.provider_id.clone())]),
            ];
            if !p.skills.is_empty() {
                lines.push(Line::from(Span::raw("")));
                lines.push(Line::from(vec![
                    label("Skills:   "),
                    Span::raw(p.skills.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(", ")),
                ]));
            }
            if !p.mcps.is_empty() {
                lines.push(Line::from(Span::raw("")));
                lines.push(Line::from(vec![
                    label("MCPs:     "),
                    Span::raw(p.mcps.iter().map(|m| m.name.as_str()).collect::<Vec<_>>().join(", ")),
                ]));
            }
            lines
        }
    };
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

use crate::domain::asset::{ProfileEntry, ProviderEntry, ScannedPackage, VaultEntry};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

const LABEL_WIDTH: usize = 13;

/// Wrap text into lines that fit within `max_width`, preserving explicit `\n`s
/// and breaking at word boundaries.
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return text.lines().map(|s| s.to_string()).collect();
    }
    let mut result = Vec::new();
    for paragraph in text.split('\n') {
        let mut current = String::new();
        for word in paragraph.split_whitespace() {
            let candidate = if current.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", current, word)
            };
            if candidate.len() <= max_width {
                current = candidate;
            } else {
                if !current.is_empty() {
                    result.push(current);
                }
                current = word.to_string();
            }
        }
        if !current.is_empty() {
            result.push(current);
        }
    }
    result
}

/// Render a labelled text block that wraps to multiple lines.
/// The first line shows `label_span` + first chunk, subsequent lines are indented.
fn push_wrapped_block<'a>(
    lines: &mut Vec<Line<'a>>,
    label_span: Span<'a>,
    content: &str,
    text_width: usize,
) {
    let wrapped = wrap_text(content, text_width);
    if wrapped.is_empty() {
        return;
    }
    lines.push(Line::from(Span::raw("")));
    let indent = " ".repeat(LABEL_WIDTH);
    for (i, text) in wrapped.iter().enumerate() {
        if i == 0 {
            lines.push(Line::from(vec![
                label_span.clone(),
                Span::raw(text.clone()),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw(indent.clone()),
                Span::raw(text.clone()),
            ]));
        }
    }
}

pub fn render(
    frame: &mut Frame,
    area: Rect,
    package: Option<&ScannedPackage>,
    is_stub: bool,
    vault_entries: &[VaultEntry],
) {
    let block = Block::default().borders(Borders::ALL).title("Detail");

    if is_stub {
        frame.render_widget(
            Paragraph::new(Line::from("  [STUB] Not yet implemented")).block(block),
            area,
        );
        return;
    }

    // Available width inside borders (left + right)
    let content_width = area.width.saturating_sub(2) as usize;
    let text_width = content_width.saturating_sub(LABEL_WIDTH);

    let lines: Vec<Line> = match package {
        None => vec![Line::from("  No item selected")],
        Some(pkg) => {
            let label = |s: &str| Span::styled(s.to_string(), Style::default().fg(Color::Yellow));
            let mut lines = vec![
                Line::from(vec![
                    label("Name:     "),
                    Span::raw(pkg.identity.name.clone()),
                ]),
                Line::from(vec![
                    label("Kind:     "),
                    Span::raw(format!("{:?}", pkg.kind)),
                ]),
                Line::from(vec![
                    label("Vault:    "),
                    Span::raw(format!(
                        "{} ({})",
                        pkg.vault_id,
                        vault_entries
                            .iter()
                            .find(|v| v.id == pkg.vault_id)
                            .map(|v| v.kind.as_str())
                            .unwrap_or("unknown")
                    )),
                ]),
                Line::from(vec![
                    label("Path:     "),
                    Span::raw(pkg.path.display().to_string()),
                ]),
            ];

            if let Some(meta) = &pkg.remote_meta {
                lines.push(Line::from(Span::raw("")));
                lines.push(Line::from(vec![
                    label("Owner:    "),
                    Span::raw(meta.owner.clone()),
                ]));
                if !meta.summary.is_empty() {
                    push_wrapped_block(
                        &mut lines,
                        Span::styled("Summary:  ", Style::default().fg(Color::Yellow)),
                        &meta.summary,
                        text_width,
                    );
                }
                lines.push(Line::from(vec![
                    label("Stats:    "),
                    Span::raw(format!(
                        "\u{2193} {}  \u{2605} {}",
                        meta.downloads, meta.stars
                    )),
                ]));
            }

            // Frontmatter metadata (PR #4)
            if let Some(author) = &pkg.author {
                lines.push(Line::from(Span::raw("")));
                lines.push(Line::from(vec![
                    label("Author:      "),
                    Span::raw(author.clone()),
                ]));
            }
            if let Some(desc) = &pkg.description {
                push_wrapped_block(
                    &mut lines,
                    Span::styled("Description: ", Style::default().fg(Color::Yellow)),
                    desc,
                    text_width,
                );
            }

            lines.push(Line::from(Span::raw("")));
            lines.push(Line::from(vec![
                label("Identity: "),
                Span::raw(pkg.identity.to_string()),
            ]));

            lines
        }
    };

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

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
    frame.render_widget(Paragraph::new(lines).block(block), area);
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
    frame.render_widget(Paragraph::new(lines).block(block), area);
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
                    Span::raw(p.skills.join(", ")),
                ]));
            }
            if !p.mcps.is_empty() {
                lines.push(Line::from(Span::raw("")));
                lines.push(Line::from(vec![
                    label("MCPs:     "),
                    Span::raw(p.mcps.join(", ")),
                ]));
            }
            lines
        }
    };
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

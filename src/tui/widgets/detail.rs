use crate::app::snapshot::VaultEntry;
use crate::domain::asset::ScannedPackage;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

// Re-export per-entity detail renderers extracted to `detail_entity.rs` so
// `crate::tui::widgets::detail::render_*_detail` keeps resolving for callers
// in `src/tui/render/content.rs`.
pub use crate::tui::widgets::detail_entity::{
    render_profile_detail, render_provider_detail, render_vault_detail,
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

/// Render a labelled text block that wraps to multiple lines for display inside
/// a `Paragraph::wrap(...)`. The first line shows `label_span` + first chunk,
/// subsequent lines are indented.
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
        let paragraph = Paragraph::new(Text::from("  [STUB] Not yet implemented"))
            .block(block)
            .wrap(Wrap { trim: false });
        frame.render_widget(paragraph, area);
        return;
    }

    // Available width inside borders (left + right)
    let content_width = area.width.saturating_sub(2) as usize;
    let text_width = content_width.saturating_sub(LABEL_WIDTH);

    let lines: Vec<Line> = match package {
        None => vec![Line::from("  No item selected")],
        Some(pkg) => {
            let label = |s: &str| Span::styled(s.to_string(), Style::default().fg(Color::Yellow));
            let mut lines: Vec<Line> = vec![
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

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

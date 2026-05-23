use crate::domain::asset::{AssetKind, ProviderEntry, ScannedPackage, VaultEntry};
use crate::domain::config::ConfigFile;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

fn truncate_name(name: &str, max_width: usize) -> String {
    if name.len() <= max_width {
        name.to_string()
    } else if max_width > 1 {
        format!("{}…", &name[..max_width - 1])
    } else {
        "…".to_string()
    }
}

fn scroll_name(name: &str, max_width: usize, offset: usize) -> String {
    if name.len() <= max_width {
        return name.to_string();
    }
    let padded = format!("{}   {}", name, name);
    let start = offset % (name.len() + 3);
    let end = (start + max_width).min(padded.len());
    padded[start..end].to_string()
}

pub fn render(
    frame: &mut Frame,
    area: Rect,
    packages: &[&ScannedPackage],
    selected: usize,
    is_stub: bool,
    config: &ConfigFile,
    scroll_offset: usize,
) {
    let block = Block::default().borders(Borders::ALL).title("Packages");

    if is_stub {
        let items = vec![ListItem::new(Line::from("  [STUB] Not yet implemented"))];
        frame.render_widget(List::new(items).block(block), area);
        return;
    }

    let inner_width = area.width.saturating_sub(2) as usize;

    // Pass 1: compute max column widths across all rows
    let mut max_owner: usize = 0;
    let mut max_dl: usize = 0;
    let mut max_star: usize = 0;
    let mut max_version: usize = 0;
    let mut max_vault: usize = 0;
    for pkg in packages.iter() {
        let ver = pkg.identity.version.as_deref().unwrap_or("--");
        max_version = max_version.max(ver.len());
        max_vault = max_vault.max(pkg.vault_id.len());
        if let Some(meta) = &pkg.remote_meta {
            max_owner = max_owner.max(meta.owner.len());
            let dl = format!("\u{2193}{}", meta.downloads);
            max_dl = max_dl.max(dl.len());
            let star = format!("\u{2605}{}", meta.stars);
            max_star = max_star.max(star.len());
        }
    }

    // Pass 2: render each row with aligned columns
    let items: Vec<ListItem> = packages
        .iter()
        .enumerate()
        .map(|(idx, pkg)| {
            let version_str = pkg.identity.version.as_deref().unwrap_or("--");
            let vault_str = &pkg.vault_id;
            let status = install_status(
                config,
                vault_str,
                &pkg.identity.name,
                &pkg.kind,
                &pkg.identity.sha10,
            );
            let is_selected = idx == selected;

            let mut spans: Vec<Span> = vec![Span::raw(format!("{} ", status))];

            if pkg.is_remote {
                let remote_style = Style::default().fg(Color::DarkGray);
                let slug = pkg
                    .identity
                    .name
                    .rsplit('/')
                    .next()
                    .unwrap_or(&pkg.identity.name);
                let (owner_str, dl_str, star_str) = if let Some(meta) = &pkg.remote_meta {
                    (
                        meta.owner.clone(),
                        format!("\u{2193}{}", meta.downloads),
                        format!("\u{2605}{}", meta.stars),
                    )
                } else {
                    (String::new(), String::new(), String::new())
                };
                // Each right column: 1 space gap + padded value
                // "  " prefix (2) per column × 5 columns = 10, plus padded content
                let right_len = (max_owner + 2)
                    + (max_dl + 2)
                    + (max_star + 2)
                    + (max_version + 2)
                    + (max_vault + 2);
                let name_budget = inner_width.saturating_sub(4 + right_len);
                let display_name = if is_selected {
                    scroll_name(slug, name_budget, scroll_offset)
                } else {
                    truncate_name(slug, name_budget)
                };
                let name_len = display_name.len();
                spans.push(Span::styled(display_name, remote_style));
                let used = 4 + name_len + right_len;
                let pad = inner_width.saturating_sub(used);
                if pad > 0 {
                    spans.push(Span::raw(" ".repeat(pad)));
                }
                spans.push(Span::styled(
                    format!("  {:<w$}", owner_str, w = max_owner),
                    remote_style,
                ));
                spans.push(Span::styled(
                    format!("  {:>w$}", dl_str, w = max_dl),
                    remote_style,
                ));
                spans.push(Span::styled(
                    format!("  {:>w$}", star_str, w = max_star),
                    remote_style,
                ));
                spans.push(Span::styled(
                    format!("  {:>w$}", version_str, w = max_version),
                    remote_style,
                ));
                spans.push(Span::styled(
                    format!("  {:<w$}", vault_str, w = max_vault),
                    remote_style,
                ));
            } else {
                // Local packages: right-align version + vault using same max widths
                let right_len = (max_version + 2) + (max_vault + 2);
                let name_budget = inner_width.saturating_sub(4 + right_len);
                let display_name = if is_selected {
                    scroll_name(&pkg.identity.name, name_budget, scroll_offset)
                } else {
                    truncate_name(&pkg.identity.name, name_budget)
                };
                let name_len = display_name.len();
                spans.push(Span::raw(display_name));
                let used = 4 + name_len + right_len;
                let pad = inner_width.saturating_sub(used);
                if pad > 0 {
                    spans.push(Span::raw(" ".repeat(pad)));
                }
                spans.push(Span::raw(format!("  {:>w$}", version_str, w = max_version)));
                spans.push(Span::raw(format!("  {:<w$}", vault_str, w = max_vault)));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let mut state = ListState::default();
    if !packages.is_empty() {
        state.select(Some(selected));
    }

    frame.render_stateful_widget(list, area, &mut state);
}

pub fn render_vaults(frame: &mut Frame, area: Rect, vaults: &[VaultEntry], selected: usize) {
    let block = Block::default().borders(Borders::ALL).title("Vaults");
    if vaults.is_empty() {
        let items = vec![ListItem::new(Line::from(
            "  No vaults attached. Press 'a' to add one.",
        ))];
        frame.render_widget(List::new(items).block(block), area);
        return;
    }
    let items: Vec<ListItem> = vaults
        .iter()
        .map(|v| {
            let check = if v.enabled { "[x]" } else { "[ ]" };
            ListItem::new(Line::from(format!(
                "{} {:<20} {:<8} {}",
                check,
                v.id,
                v.kind,
                v.counts_label()
            )))
        })
        .collect();
    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    let mut state = ListState::default();
    if !vaults.is_empty() {
        state.select(Some(selected));
    }
    frame.render_stateful_widget(list, area, &mut state);
}

pub fn render_providers(
    frame: &mut Frame,
    area: Rect,
    providers: &[ProviderEntry],
    selected: usize,
) {
    let block = Block::default().borders(Borders::ALL).title("Providers");
    if providers.is_empty() {
        let items = vec![ListItem::new(Line::from("  No providers installed."))];
        frame.render_widget(List::new(items).block(block), area);
        return;
    }
    let items: Vec<ListItem> = providers
        .iter()
        .map(|p| {
            let checkbox = if p.active { "[x]" } else { "[ ]" };
            ListItem::new(Line::from(format!("{} {}", checkbox, p.name)))
        })
        .collect();
    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    let mut state = ListState::default();
    if !providers.is_empty() {
        state.select(Some(selected));
    }
    frame.render_stateful_widget(list, area, &mut state);
}

fn install_status(
    config: &ConfigFile,
    vault_id: &str,
    name: &str,
    kind: &AssetKind,
    current_hash: &str,
) -> &'static str {
    match kind {
        AssetKind::Skill => {
            if let Some(installed_hash) = config.installed_skill_hash(vault_id, name) {
                if installed_hash != current_hash {
                    "[!]"
                } else {
                    "[✓]"
                }
            } else {
                "[ ]"
            }
        }
        AssetKind::Instruction => {
            if let Some(installed_hash) = config.installed_instruction_hash(vault_id, name) {
                if installed_hash != current_hash {
                    "[!]"
                } else {
                    "[✓]"
                }
            } else {
                "[ ]"
            }
        }
        AssetKind::McpServer => "[MCP]",
    }
}

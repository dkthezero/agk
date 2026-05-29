//! Per-entity list renderers (vaults, providers, profiles).
//!
//! Extracted from `list.rs` to keep that file under the 300-LOC ADR-001 §6.4
//! limit. Callers go through `crate::tui::widgets::list::*` (re-exported by
//! `list.rs`) so no consumer needs to change its import paths.

use crate::app::snapshot::{ProfileEntry, ProviderEntry, VaultEntry};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

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

pub fn render_profiles(frame: &mut Frame, area: Rect, profiles: &[ProfileEntry], selected: usize) {
    let block = Block::default().borders(Borders::ALL).title("Profiles");
    if profiles.is_empty() {
        let items = vec![ListItem::new(Line::from(
            "  No profiles. Press F2 to add one.",
        ))];
        frame.render_widget(List::new(items).block(block), area);
        return;
    }
    let items: Vec<ListItem> = profiles
        .iter()
        .map(|p| ListItem::new(Line::from(format!("{} ({})", p.name, p.provider_id))))
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
    if !profiles.is_empty() {
        state.select(Some(selected));
    }
    frame.render_stateful_widget(list, area, &mut state);
}

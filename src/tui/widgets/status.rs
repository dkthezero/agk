use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Parameters for the status bar rendering.
pub struct StatusParams<'a> {
    pub status: &'a str,
    pub search: &'a str,
    pub keybinds: &'a str,
    pub scope_label: &'a str,
    pub progress: Option<&'a str>,
    pub team_status: Option<&'a str>,
}

pub fn render(frame: &mut Frame, area: Rect, params: StatusParams<'_>) {
    let StatusParams {
        status,
        search,
        keybinds,
        scope_label,
        progress,
        team_status,
    } = params;
    let status_text = if !status.is_empty() {
        status.to_string()
    } else if !search.is_empty() {
        format!("Search: {}", search)
    } else {
        String::new()
    };

    let mut line_spans: Vec<Span> = if keybinds.is_empty() {
        vec![Span::styled(scope_label, Style::default().fg(Color::Green))]
    } else {
        std::iter::once(Span::styled(scope_label, Style::default().fg(Color::Green)))
            .chain(std::iter::once(Span::raw("  ")))
            .chain(color_keybinds(keybinds))
            .collect()
    };

    // Append team status badge if available
    if let Some(ts) = team_status {
        if !ts.is_empty() {
            line_spans.push(Span::raw("  "));
            line_spans.push(Span::styled(
                ts.to_string(),
                Style::default().fg(Color::Cyan),
            ));
        }
    }

    let line1 = Line::from(line_spans);

    let row_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(1),
            ratatui::layout::Constraint::Length(1),
        ])
        .split(area);

    frame.render_widget(Paragraph::new(line1), row_layout[0]);

    if let Some(prog) = progress {
        let bottom_layout = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([
                ratatui::layout::Constraint::Min(0),
                ratatui::layout::Constraint::Length(prog.len() as u16 + 2),
            ])
            .split(row_layout[1]);

        frame.render_widget(Paragraph::new(status_text), bottom_layout[0]);
        frame.render_widget(
            Paragraph::new(prog)
                .alignment(ratatui::layout::Alignment::Right)
                .style(Style::default().fg(Color::Yellow)),
            bottom_layout[1],
        );
    } else {
        frame.render_widget(Paragraph::new(status_text), row_layout[1]);
    }
}

/// Parse a keybinds string like "[↑/↓] Move  [Space] Toggle" into colored spans.
/// Keys (inside `[]`) are shown in Cyan; everything else is DarkGray.
fn color_keybinds(input: &str) -> Vec<Span<'_>> {
    let mut spans = Vec::new();
    let mut current = String::new();
    let mut in_bracket = false;

    for ch in input.chars() {
        if ch == '[' && !in_bracket {
            if !current.is_empty() {
                spans.push(Span::styled(
                    current.clone(),
                    Style::default().fg(Color::DarkGray),
                ));
                current.clear();
            }
            in_bracket = true;
            current.push(ch);
        } else if ch == ']' && in_bracket {
            current.push(ch);
            spans.push(Span::styled(
                current.clone(),
                Style::default().fg(Color::Cyan),
            ));
            current.clear();
            in_bracket = false;
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        let color = if in_bracket {
            Color::Cyan
        } else {
            Color::DarkGray
        };
        spans.push(Span::styled(current, Style::default().fg(color)));
    }
    spans
}

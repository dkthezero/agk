use crate::domain::config::vault_section::AssetSource;
use ratatui::text::Span;

/// Render a `[Team]` badge for the TUI.
///
/// Returns a cyan-styled ` [Team] ` span when the source is `Team`,
/// or an empty span when `Personal`.
pub fn team_badge(source: &AssetSource) -> Span<'static> {
    match source {
        AssetSource::Team => Span::styled(
            " [Team] ",
            ratatui::style::Style::default().fg(ratatui::style::Color::Cyan),
        ),
        AssetSource::Personal => Span::raw(""),
    }
}

/// Render the team status bar line: `[Team] X/Y check | Z personal`
pub fn team_status_line(installed: usize, required: usize, personal: usize) -> String {
    let check = if installed == required { "\u{2713}" } else { "\u{2717}" };
    format!(
        "[Team] {}/{} {} | {} personal",
        installed, required, check, personal
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn team_badge_shows_cyan_for_team_source() {
        let badge = team_badge(&AssetSource::Team);
        assert_eq!(badge.content, " [Team] ");
        assert!(badge.style.fg == Some(ratatui::style::Color::Cyan));
    }

    #[test]
    fn team_badge_is_empty_for_personal_source() {
        let badge = team_badge(&AssetSource::Personal);
        assert!(badge.content.is_empty());
    }

    #[test]
    fn team_status_line_all_installed() {
        let line = team_status_line(15, 15, 3);
        assert_eq!(line, "[Team] 15/15 \u{2713} | 3 personal");
    }

    #[test]
    fn team_status_line_missing_requirements() {
        let line = team_status_line(12, 15, 5);
        assert_eq!(line, "[Team] 12/15 \u{2717} | 5 personal");
    }

    #[test]
    fn team_status_line_zero_required() {
        let line = team_status_line(0, 0, 10);
        assert_eq!(line, "[Team] 0/0 \u{2713} | 10 personal");
    }
}
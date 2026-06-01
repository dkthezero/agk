/// Estimate token count from text using a `words * 1.35` heuristic.
///
/// Average English word ≈ 1.35 tokens. Labelled "Est." in UI;
/// not a hard limit.
pub fn estimate_tokens(text: &str) -> usize {
    let word_count = text.split_whitespace().count();
    (word_count as f32 * 1.35) as usize
}

/// Return a color label based on token count thresholds.
///
/// - Green: < 500
/// - Yellow: 500–800
/// - Red: > 800
pub fn token_badge_color(tokens: usize) -> &'static str {
    if tokens < 500 {
        "green"
    } else if tokens <= 800 {
        "yellow"
    } else {
        "red"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_hello_world() {
        // "hello world" = 2 words ≈ 2.7 tokens => 2 after truncation
        assert_eq!(estimate_tokens("hello world"), 2);
    }

    #[test]
    fn estimate_empty() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn estimate_large_text() {
        let text = "The quick brown fox jumps over the lazy dog. ";
        let text = text.repeat(10); // 90 words
        assert_eq!(estimate_tokens(&text), 121); // 90 * 1.35 = 121.5 => 121
    }

    #[test]
    fn badge_green() {
        assert_eq!(token_badge_color(499), "green");
    }

    #[test]
    fn badge_yellow() {
        assert_eq!(token_badge_color(500), "yellow");
        assert_eq!(token_badge_color(800), "yellow");
    }

    #[test]
    fn badge_red() {
        assert_eq!(token_badge_color(801), "red");
    }
}

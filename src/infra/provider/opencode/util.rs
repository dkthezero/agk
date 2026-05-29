/// Generate a 6-digit numeric suffix based on nanosecond time.
///
/// Shared by `session.rs` and `legacy_session.rs` to mint per-session
/// agent names.
pub fn random_6_digits() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:06}", nanos % 1_000_000)
}

/// Patch frontmatter in an agent markdown to set the correct name and mode.
pub fn patch_agent_frontmatter(content: &str, agent_name: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut in_frontmatter = false;
    let mut frontmatter_started = false;
    let mut result = Vec::new();
    let mut name_set = false;
    let mut mode_set = false;

    for line in &lines {
        if *line == "---" {
            if !frontmatter_started {
                frontmatter_started = true;
                in_frontmatter = true;
                result.push(line.to_string());
                continue;
            } else if in_frontmatter {
                in_frontmatter = false;
                if !name_set {
                    result.push(format!("name: {}", agent_name));
                }
                if !mode_set {
                    result.push("mode: primary".to_string());
                }
                result.push(line.to_string());
                continue;
            }
        }

        if in_frontmatter {
            if line.starts_with("name:") {
                result.push(format!("name: {}", agent_name));
                name_set = true;
                continue;
            }
            if line.starts_with("mode:") {
                result.push("mode: primary".to_string());
                mode_set = true;
                continue;
            }
        }

        result.push(line.to_string());
    }

    // If no frontmatter at all, prepend one
    if !frontmatter_started {
        result.insert(0, "---".to_string());
        result.insert(1, format!("name: {}", agent_name));
        result.insert(2, "mode: primary".to_string());
        result.insert(3, "---".to_string());
    }

    result.join("\n") + "\n"
}

// ---------------------------------------------------------------------------
// JSONC comment stripper (basic)
// ---------------------------------------------------------------------------

pub fn strip_jsonc_comments(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();
    let mut in_string = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;

    while let Some(ch) = chars.next() {
        if in_line_comment {
            if ch == '\n' {
                in_line_comment = false;
                result.push('\n');
            }
            continue;
        }

        if in_block_comment {
            if ch == '*' {
                if let Some(&'/') = chars.peek() {
                    chars.next();
                    in_block_comment = false;
                }
            }
            continue;
        }

        if in_string {
            result.push(ch);
            if ch == '\\' {
                if let Some(next) = chars.next() {
                    result.push(next);
                }
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            result.push(ch);
            continue;
        }

        if ch == '/' {
            match chars.peek() {
                Some(&'/') => {
                    chars.next();
                    in_line_comment = true;
                    continue;
                }
                Some(&'*') => {
                    chars.next();
                    in_block_comment = true;
                    continue;
                }
                _ => {}
            }
        }

        result.push(ch);
    }

    result
}

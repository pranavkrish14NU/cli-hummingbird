// Unified diff utilities — used by ForgeEngine::apply_edit and ForgeEngine::undo.
// Additional diff formatting helpers can be added here.

pub fn colorize_diff(diff: &str) -> String {
    diff.lines().map(|line| {
        if line.starts_with('+') && !line.starts_with("+++") {
            format!("\x1b[32m{line}\x1b[0m") // green
        } else if line.starts_with('-') && !line.starts_with("---") {
            format!("\x1b[31m{line}\x1b[0m") // red
        } else if line.starts_with("@@") {
            format!("\x1b[36m{line}\x1b[0m") // cyan
        } else {
            line.to_string()
        }
    }).collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colorizes_added_lines() {
        let diff = "+fn added() {}";
        let colored = colorize_diff(diff);
        assert!(colored.contains("\x1b[32m"));
    }

    #[test]
    fn colorizes_removed_lines() {
        let diff = "-fn removed() {}";
        let colored = colorize_diff(diff);
        assert!(colored.contains("\x1b[31m"));
    }

    #[test]
    fn passes_through_context_lines() {
        let diff = " context line";
        let colored = colorize_diff(diff);
        assert_eq!(colored, " context line");
    }
}

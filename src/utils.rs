//! shared utilities: spinners, etc

use std::path::Path;

/// Classic Braille Spinner -- should work anywhere
pub const SPINNER_BRAILLE: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Dots Variant Spinner -- should work anywhere
#[allow(dead_code)]
pub const SPINNER_DOTS: &[&str] = &["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"];

/// Claude Code Spinner(macOS)
#[allow(dead_code)]
pub const SPINNER_CC_MACOS: &[&str] = &[".", "+", "*", "*", "*", "*"];

/// ASCII Spinner -- for terminal that
#[allow(dead_code)]
pub const SPINNER_ASCII: &[&str] = &["-", "\\", "|", "/"];

pub fn spinner_frame(tick: usize) -> &'static str {
    let spinner = SPINNER_BRAILLE;
    spinner[tick % spinner.len()]
}

/*
 * helper functions to get project name
 */
pub fn detect_project_name() -> String {
    detect_project_name_at(&std::env::current_dir().unwrap_or_else(|_| ".".into()))
}

pub fn detect_project_name_at(base: &Path) -> String {
    read_cargo_name_at(base)
        .or_else(|| read_package_json_name_at(base))
        .or_else(|| read_pyproject_name_at(base))
        .or_else(|| read_dir_basename_at(base))
        .unwrap_or_else(|| "unknown".into())
}

/* detect project type */
pub fn detect_project_type() -> String {
    detect_project_type_at(&std::env::current_dir().unwrap_or_else(|_| ".".into()))
}

pub fn read_readme_summary(max_chars: usize) -> String {
    read_readme_summary_at(
        &std::env::current_dir().unwrap_or_else(|_| ".".into()),
        max_chars,
    )
}

pub fn read_readme_summary_at(base: &Path, max_chars: usize) -> String {
    let content = match std::fs::read_to_string(base.join("README.md")) {
        Ok(s) => s,
        Err(_) => return String::new(),
    };
    content.chars().take(max_chars).collect()
}

pub fn read_agents_md() -> Option<String> {
    read_agents_md_at(&std::env::current_dir().unwrap_or_else(|_| ".".into()))
}

pub fn read_agents_md_at(base: &Path) -> Option<String> {
    std::fs::read_to_string(base.join("AGENTS.md")).ok()
}

pub fn detect_project_type_at(base: &Path) -> String {
    if base.join("Cargo.toml").exists() {
        "rust".into()
    } else if base.join("package.json").exists() {
        "node".into()
    } else if base.join("pyproject.toml").exists() {
        "python".into()
    } else {
        "unknown".into()
    }
}

// detect the project name(Rust)
fn read_cargo_name_at(base: &Path) -> Option<String> {
    let content = std::fs::read_to_string(base.join("Cargo.toml")).ok()?;

    content
        .lines()
        .find(|line| line.trim_start().starts_with("name"))
        .and_then(|line| line.split('=').nth(1))
        .map(|s| s.trim().trim_matches('"').to_string())
}

// detects the project name(Node)
fn read_package_json_name_at(base: &Path) -> Option<String> {
    let content = std::fs::read_to_string(base.join("package.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&content).ok()?;
    v.get("name")?.as_str().map(String::from)
}

// detects the project name(Python)
fn read_pyproject_name_at(base: &Path) -> Option<String> {
    let content = std::fs::read_to_string(base.join("pyproject.toml")).ok()?;

    content
        .lines()
        .find(|line| line.trim_start().starts_with("name"))
        .and_then(|line| line.split('=').nth(1))
        .map(|s| s.trim().trim_matches('"').to_string())
}

// detects the project name(fallback)
fn read_dir_basename_at(base: &Path) -> Option<String> {
    base.file_name()?.to_str().map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn spinner_frame_cycle() {
        let len = SPINNER_BRAILLE.len();
        assert_eq!(spinner_frame(0), SPINNER_BRAILLE[0]);
        assert_eq!(spinner_frame(len), SPINNER_BRAILLE[0]);
        assert_eq!(spinner_frame(len + 1), SPINNER_BRAILLE[1]);
    }

    #[test]
    fn detect_project_type_rust() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"foo\"\n").unwrap();
        assert_eq!(detect_project_type_at(dir.path()), "rust");
    }

    #[test]
    fn detect_project_type_node() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("package.json"), r#"{"name":"web-app"}"#).unwrap();
        assert_eq!(detect_project_type_at(dir.path()), "node");
    }

    #[test]
    fn detect_project_type_python() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            "[project]\nname = \"py-app\"\n",
        )
        .unwrap();
        assert_eq!(detect_project_type_at(dir.path()), "python");
    }

    #[test]
    fn detect_project_type_unknown() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(detect_project_type_at(dir.path()), "unknown");
    }

    #[test]
    fn read_cargo_name_parses_name() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"my-crate\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        assert_eq!(read_cargo_name_at(dir.path()), Some("my-crate".into()));
    }

    #[test]
    fn read_package_json_name_parses_name() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("package.json"), r#"{"name":"web-app"}"#).unwrap();
        assert_eq!(
            read_package_json_name_at(dir.path()),
            Some("web-app".into())
        );
    }

    #[test]
    fn read_pyproject_name_parses_name() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            "[project]\nname = \"py-app\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        assert_eq!(read_pyproject_name_at(dir.path()), Some("py-app".into()));
    }

    #[test]
    fn detect_project_name_prefers_manifest_name() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"my-crate\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        assert_eq!(detect_project_name_at(dir.path()), "my-crate");
    }

    #[test]
    fn detect_project_name_falls_back_to_dir_name() {
        let dir = tempfile::tempdir().unwrap();
        let expected = dir.path().file_name().unwrap().to_str().unwrap();
        assert_eq!(detect_project_name_at(dir.path()), expected);
    }

    #[test]
    fn read_readme_summary_truncates() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("README.md"), "a".repeat(1000)).unwrap();

        let summary = read_readme_summary_at(dir.path(), 100);

        assert_eq!(summary.len(), 100);
    }

    #[test]
    fn read_readme_summary_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let summary = read_readme_summary_at(dir.path(), 100);

        assert!(summary.is_empty());
    }

    #[test]
    fn read_agents_md_reads_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("AGENTS.md"), "Follow project instructions.").unwrap();

        assert_eq!(
            read_agents_md_at(dir.path()),
            Some("Follow project instructions.".into())
        );
    }

    #[test]
    fn read_agents_md_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(read_agents_md_at(dir.path()), None);
    }
}

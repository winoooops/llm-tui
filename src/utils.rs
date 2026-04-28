//! shared utilities: spinners, etc

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
    read_cargo_name()
        .or_else(read_package_json_name)
        .or_else(read_pyproject_name)
        .or_else(read_dir_basename)
        .unwrap_or_else(|| "unknown".into())
}

/* detect project type */
pub fn detect_project_type() -> String {
    if std::path::Path::new("Cargo.toml").exists() {
        "rust".into()
    } else if std::path::Path::new("package.json").exists() {
        "node".into()
    } else if std::path::Path::new("pyproject.toml").exists() {
        "python".into()
    } else {
        "unknown".into()
    }
}

pub fn read_readme_summary(max_chars: usize) -> String {
    let content = match std::fs::read_to_string("README.md") {
        Ok(s) => s,
        Err(_) => return String::new(),
    };
    content.chars().take(max_chars).collect()
}

pub fn read_agents_md() -> Option<String> {
    std::fs::read_to_string("AGENTS.md").ok()
}


// detect the project name(Rust)
fn read_cargo_name() -> Option<String> {
    let content = std::fs::read_to_string("Cargo.toml").ok()?;

    content.lines()
        .find(|line| line.trim_start().starts_with("name"))
        .and_then(|line| line.split('=').nth(1))
        .map(|s| s.trim().trim_matches('"').to_string())
}

// detects the project name(Node)
fn read_package_json_name() -> Option<String> {
    let content = std::fs::read_to_string("package.json").ok()?;
    let v: serde_json::Value = serde_json::from_str(&content).ok()?;
    v.get("name")?.as_str().map(String::from)
}

// detects the project name(Python)
fn read_pyproject_name() -> Option<String> {
    let content = std::fs::read_to_string("pyproject.toml").ok()?;

    content.lines()
        .find(|line| line.trim_start().starts_with("name"))
        .and_then(|line| line.split('=').nth(1))
        .map(|s| s.trim().trim_matches('"').to_string())
}

// detects the project name(fallback)
fn read_dir_basename() -> Option<String> {
    std::env::current_dir().ok()?
        .file_name()?
        .to_str()
        .map(String::from)
}



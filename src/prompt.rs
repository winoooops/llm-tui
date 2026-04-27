use std::path::Path;

pub struct PromptContext {
    pub cwd: String,
    pub project_name: String,
    pub project_summary: String,
    pub agents_md: Option<String>,
}


const README_SUMMARY_MAX_CHARS: usize = 500; // ~125 tokens, control the length of system prompt

impl PromptContext {
    pub fn from_environment() -> Self {
        let cwd = std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| ".".into());

        let project_name = read_cargo
    }




    pub fn new(cwd: &str, project_name: &str, project_summary: &str, agents_md: Option<&str>) -> Self {
        Self {
            cwd: cwd.into(),
            project_name: project_name.into(),
            project_summary: project_summary.into(),
            agents_md: agents_md.map(|s| s.into())
        }
    }

    // detect the project name(Rust)
    fn read_cargo_name() -> Option<String> {
        let content = std::fs::read_to_string("Cargo.toml").ok()?;

        content.lines()
            .find(|line| line.trim_start().starts_with("name"))
            .and_then(|line| line.split('=').nth(1))
            .map(|s| s.trim().trim_matches('"').to_string())
    }

    fn read_package_json_name() -> Option<String> {
        let content = std::fs::read_to_string("package.json").ok()?;
        let v: serde_json::Value = serde_json::from_str(&content).ok()?;
        v.get("name")?.as_str().map(String::from)
    }

}

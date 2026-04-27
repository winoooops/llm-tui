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
    }




    pub fn new(cwd: &str, project_name: &str, project_summary: &str, agents_md: Option<&str>) -> Self {
        Self {
            cwd: cwd.into(),
            project_name: project_name.into(),
            project_summary: project_summary.into(),
            agents_md: agents_md.map(|s| s.into())
        }
    }

}

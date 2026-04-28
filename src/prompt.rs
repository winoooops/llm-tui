use crate::{
    message::Message,
    utils::{detect_project_name, detect_project_type, read_agents_md, read_readme_summary},
};

pub struct PromptContext {
    pub cwd: String,
    pub project_name: String,
    pub project_summary: String,
    pub project_type: String,
    pub agents_md: Option<String>,
}

const README_SUMMARY_MAX_CHARS: usize = 500; // ~125 tokens, control the length of system prompt
const DEFAULT_SYSTEM_PROMPT: &str = "You are a helpful coding assistant";
const BOUNDARY: &str = "__SYSTEM_PROMPT_DYNAMIC_BOUNDARY__";

impl PromptContext {
    #[allow(dead_code)] // 供测试直接构造，主流程用 from_environment()
    pub fn new(
        cwd: &str,
        project_name: &str,
        project_summary: &str,
        project_type: &str,
        agents_md: Option<&str>,
    ) -> Self {
        Self {
            cwd: cwd.into(),
            project_name: project_name.into(),
            project_type: project_type.into(),
            project_summary: project_summary.into(),
            agents_md: agents_md.map(|s| s.into()),
        }
    }

    /* collet the context from local env */
    pub fn from_environment() -> Self {
        Self::from_path(&std::env::current_dir().unwrap_or_else(|_| ".".into()))
    }

    pub fn from_path(base: &std::path::Path) -> Self {
        let cwd = base.to_string_lossy().to_string();
        let project_name = detect_project_name();
        let project_type = detect_project_type();
        let project_summary = read_readme_summary(README_SUMMARY_MAX_CHARS);
        let agents_md = read_agents_md();

        Self {
            cwd,
            project_name,
            project_type,
            project_summary,
            agents_md,
        }
    }

    /* return the system prompt */
    pub fn system_prompt(&self) -> Message {
        let static_prompt = load_static_prompt();
        self.assemble_system_message(&static_prompt)
    }

    fn assemble_system_message(&self, static_prompt: &str) -> Message {
        let mut dynamic = format!(
            "# Environment\n\
            - Working Directory: {}\n\
            - Project: {}\n\
            - Project Type: {}\n\
            ",
            &self.cwd, &self.project_name, &self.project_type
        );

        if !&self.project_summary.is_empty() {
            dynamic.push_str(&format!("\n# Project Summary\n{}\n", &self.project_summary));
        }

        if let Some(agents) = &self.agents_md {
            dynamic.push_str(&format!(
                "\n# project instructions(agents.md)\n{}\n",
                agents
            ));
        }

        let full = format!(
            "{}\n\n{}\n\n{}",
            static_prompt.trim(),
            BOUNDARY,
            dynamic.trim()
        );

        Message::system(full)
    }
}

fn load_static_prompt() -> String {
    std::fs::read_to_string(".prompts/system.md").unwrap_or_else(|_| DEFAULT_SYSTEM_PROMPT.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn system_prompt_contains_boundary() {
        let ctx = PromptContext::new(
            "/projects/foo",
            "bar",
            "test project",
            "rust",
            Some("be helpful"),
        );

        let msg = ctx.system_prompt();
        assert_eq!(msg.role, "system");
        assert!(msg.content.contains(BOUNDARY))
    }

    #[test]
    fn system_prompt_omits_empty_summary() {
        let ctx = PromptContext::new("/tmp", "x", "", "rust", None);
        let msg = ctx.system_prompt();
        assert!(!msg.content.contains("# project summary"));
    }

    #[test]
    fn system_prompt_includes_agents_md() {
        let ctx = PromptContext::new("/tmp", "x", "summary", "rust", Some("be aggressive"));
        let msg = ctx.system_prompt();
        assert!(msg.content.contains("be aggressive"));
        assert!(msg.content.contains("project instructions(agents.md)"));
    }

    #[test]
    fn new_constructor_maps_fields() {
        let ctx = PromptContext::new("/a", "b", "c", "d", Some("e"));
        assert_eq!(ctx.cwd, "/a");
        assert_eq!(ctx.project_name, "b");
        assert_eq!(ctx.project_summary, "c");
        assert_eq!(ctx.project_type, "d");
        assert_eq!(ctx.agents_md, Some("e".into()));
    }
}

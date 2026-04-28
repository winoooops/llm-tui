use crate::{message::Message, utils::{detect_project_name, detect_project_type, read_agents_md, read_readme_summary}};

pub struct PromptContext {
    pub cwd: String,
    pub project_name: String,
    pub project_summary: String,
    pub project_type: String,
    pub agents_md: Option<String>,
}


const README_SUMMARY_MAX_CHARS: usize = 500; // ~125 tokens, control the length of system prompt
const DEFAULT_SYSTEM_PROMPT: &str = "You are a helpful coding assistant";

impl PromptContext {
    /* collet the context from local env */
    pub fn from_environment() -> Self {
        let cwd = std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| ".".into());

        let project_name = detect_project_name();
        let project_type = detect_project_type();
        let project_summary = read_readme_summary(README_SUMMARY_MAX_CHARS);
        let agents_md = read_agents_md();

        Self {
            cwd,
            project_name,
            project_type,
            project_summary,
            agents_md
        }
    }

    /* collect the context from arguments */
    pub fn new(cwd: &str, project_name: &str, project_summary: &str, project_type: &str, agents_md: Option<&str>) -> Self {
        Self {
            cwd: cwd.into(),
            project_name: project_name.into(),
            project_type: project_type.into(),
            project_summary: project_summary.into(),
            agents_md: agents_md.map(|s| s.into())
        }
    }

    /* return the system prompt */
    pub fn system_prompt(&self) -> Message {
        let static_prompt = load_static_prompt();
        self.assemble_system_message(&static_prompt)
    }

    fn assemble_system_message(&self, static_prompt: &str) -> Message {
        const BOUNDARY: &str = "__SYSTEM_PROMPT_DYNAMIC_BOUNDARY__";

        let mut dynamic = format!(
            "# Environment\n\
            - Working Directory: {}\n\
            - Project: {}\n\
            - Project Type: {}\n\
            ",
            &self.cwd, &self.project_name, &self.project_type
        );

        if !&self.project_summary.is_empty() {
            dynamic.push_str(&format!(
                    "\n# Project Summary\n{}\n",
                    &self.project_summary
            ));
        }

        if let Some(agents) = &self.agents_md {
            dynamic.push_str(&format!(
                    "\n# Project Instructions(Agents.md)\n{}\n",
                    agents
            ));
        }

        let full = format!("{}\n\n{}\n\n{}", static_prompt.trim(), BOUNDARY, dynamic.trim());

        Message::system(full)
    }
}


fn load_static_prompt() -> String {
    std::fs::read_to_string(".prompts/system.md")
        .unwrap_or_else(|_| DEFAULT_SYSTEM_PROMPT.into())
}





#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
  pub fn user(content: impl Into<String>) -> Self {
      Self {
          role: "user".into(),
          content: content.into()
      }
  }

  pub fn assistant(content: impl Into<String>) -> Self {
      Self {
          role: "assistant".into(),
          content: content.into()
      }
  }
}



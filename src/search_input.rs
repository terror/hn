pub(crate) struct SearchInput {
  pub(crate) buffer: String,
  pub(crate) message_backup: String,
}

impl SearchInput {
  pub(crate) fn new(message_backup: String) -> Self {
    Self {
      buffer: String::new(),
      message_backup,
    }
  }

  pub(crate) fn prompt(&self) -> String {
    format!("Search: {}", self.buffer)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn prompt_reflects_current_buffer() {
    let mut input = SearchInput::new("status".to_string());
    assert_eq!(input.prompt(), "Search: ");

    input.buffer.push_str("rust");
    assert_eq!(input.prompt(), "Search: rust");
  }
}

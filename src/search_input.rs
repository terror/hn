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

use super::*;

#[derive(Clone)]
pub(crate) struct TransientMessage {
  current: String,
  expires_at: Instant,
  original: String,
}

impl TransientMessage {
  pub(crate) fn current(&self) -> &str {
    &self.current
  }

  pub(crate) fn is_expired(&self) -> bool {
    Instant::now() >= self.expires_at
  }

  pub(crate) fn new(current: String, original: String) -> Self {
    Self {
      expires_at: Instant::now() + Duration::from_secs(3),
      current,
      original,
    }
  }

  pub(crate) fn original(&self) -> &str {
    &self.original
  }
}

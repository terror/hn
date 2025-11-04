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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn new_sets_fields_and_expiry_three_seconds_ahead() {
    let now = Instant::now();
    let message = TransientMessage::new("current".into(), "original".into());

    assert_eq!(message.current(), "current");
    assert_eq!(message.original(), "original");

    let remaining = message.expires_at.duration_since(now);
    assert!(remaining >= Duration::from_secs(3));
    assert!(remaining <= Duration::from_secs(3) + Duration::from_millis(10));
  }

  #[test]
  fn is_expired_detects_elapsed_time() {
    let mut message = TransientMessage::new("a".into(), "b".into());

    assert!(!message.is_expired());

    message.expires_at =
      Instant::now().checked_sub(Duration::from_secs(1)).unwrap();

    assert!(message.is_expired());
  }
}

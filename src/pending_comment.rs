use super::*;

pub(crate) struct PendingComment {
  pub(crate) fallback_link: String,
  pub(crate) request_id: u64,
}

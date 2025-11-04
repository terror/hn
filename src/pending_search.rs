use super::*;

pub(crate) struct PendingSearch {
  pub(crate) query: String,
  pub(crate) request_id: u64,
  pub(crate) tab_index: usize,
}

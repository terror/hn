use super::*;

pub(crate) enum Event {
  Comments {
    request_id: u64,
    result: Result<CommentThread>,
  },
  SearchResults {
    request_id: u64,
    result: Result<(Vec<ListEntry>, bool)>,
  },
  TabItems {
    tab_index: usize,
    result: Result<Vec<ListEntry>>,
  },
}

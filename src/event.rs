use super::*;

pub(crate) enum Event {
  CommentsLoaded {
    request_id: u64,
    result: Result<CommentThread>,
  },
  TabItemsLoaded {
    tab_index: usize,
    result: Result<Vec<ListEntry>>,
  },
  SearchResultsLoaded {
    request_id: u64,
    result: Result<(Vec<ListEntry>, bool)>,
  },
}

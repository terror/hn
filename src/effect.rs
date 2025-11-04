use super::category::Category;

#[derive(Clone)]
pub(crate) enum Effect {
  FetchComments {
    item_id: u64,
    request_id: u64,
  },
  FetchSearchResults {
    query: String,
    request_id: u64,
  },
  FetchTabItems {
    tab_index: usize,
    category: Category,
    offset: usize,
  },
  OpenUrl {
    url: String,
  },
}

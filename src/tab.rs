use super::*;

pub(crate) struct Tab {
  pub(crate) category: Category,
  pub(crate) has_more: bool,
  pub(crate) items: Vec<Entry>,
  pub(crate) label: &'static str,
  pub(crate) offset: usize,
  pub(crate) selected: usize,
}

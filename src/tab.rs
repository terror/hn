use super::*;

pub(crate) struct Tab {
  pub(crate) category: Category,
  pub(crate) has_more: bool,
  pub(crate) label: &'static str,
}

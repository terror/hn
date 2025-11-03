use super::*;

#[derive(Clone, Debug)]
pub(crate) struct CommentThread {
  pub(crate) focus: Option<u64>,
  pub(crate) roots: Vec<Comment>,
  pub(crate) url: Option<String>,
}

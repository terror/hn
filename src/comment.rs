#[derive(Clone, Debug)]
pub(crate) struct Comment {
  pub(crate) author: Option<String>,
  pub(crate) children: Vec<Comment>,
  pub(crate) dead: bool,
  pub(crate) deleted: bool,
  pub(crate) id: u64,
  pub(crate) text: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct CommentThread {
  pub(crate) focus: Option<u64>,
  pub(crate) roots: Vec<Comment>,
  pub(crate) title: String,
  pub(crate) url: Option<String>,
}

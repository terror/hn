#[derive(Clone, Debug)]
pub(crate) struct Comment {
  pub(crate) author: Option<String>,
  pub(crate) children: Vec<Comment>,
  pub(crate) dead: bool,
  pub(crate) deleted: bool,
  pub(crate) id: u64,
  pub(crate) text: Option<String>,
}

pub(crate) struct CommentEntry {
  pub(crate) author: Option<String>,
  pub(crate) body: String,
  pub(crate) children: Vec<usize>,
  pub(crate) dead: bool,
  pub(crate) deleted: bool,
  pub(crate) depth: usize,
  pub(crate) expanded: bool,
  pub(crate) parent: Option<usize>,
}

impl CommentEntry {
  pub(crate) fn body(&self) -> &str {
    self.body.as_str()
  }

  pub(crate) fn has_children(&self) -> bool {
    !self.children.is_empty()
  }

  pub(crate) fn header(&self) -> String {
    let author = self.author.as_deref().unwrap_or("unknown");

    match (self.deleted, self.dead) {
      (true, _) => format!("{author} (deleted)"),
      (_, true) => format!("{author} (dead)"),
      _ => author.to_string(),
    }
  }
}

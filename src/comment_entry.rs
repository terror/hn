use super::*;

pub(crate) struct CommentEntry {
  pub(crate) author: Option<String>,
  pub(crate) body: String,
  pub(crate) children: Vec<usize>,
  pub(crate) dead: bool,
  pub(crate) deleted: bool,
  pub(crate) depth: usize,
  pub(crate) expanded: bool,
  pub(crate) id: u64,
  pub(crate) parent: Option<usize>,
}

impl CommentEntry {
  pub(crate) fn body(&self) -> &str {
    self.body.as_str()
  }

  pub(crate) fn permalink(&self) -> String {
    format!("https://news.ycombinator.com/item?id={}", self.id)
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

  pub(crate) fn to_bookmark_entry(&self) -> ListEntry {
    let author = self.author.as_deref().unwrap_or("unknown");
    let title = format!("Comment by {author}");

    let mut snippet = String::new();
    let mut char_count: usize = 0;

    for word in self.body().split_whitespace() {
      if !snippet.is_empty() {
        snippet.push(' ');
        char_count = char_count.saturating_add(1);
      }

      snippet.push_str(word);
      char_count = char_count.saturating_add(word.chars().count());

      if char_count >= 120 {
        break;
      }
    }

    let detail = {
      let trimmed = snippet.trim();

      if trimmed.is_empty() {
        None
      } else {
        Some(truncate(trimmed, 120))
      }
    };

    ListEntry {
      detail,
      id: self.id.to_string(),
      title,
      url: Some(self.permalink()),
    }
  }
}

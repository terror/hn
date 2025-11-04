use super::*;

pub(crate) struct CommentView {
  pub(crate) entries: Vec<CommentEntry>,
  pub(crate) link: String,
  pub(crate) offset: usize,
  pub(crate) selected: Option<usize>,
}

impl CommentView {
  pub(crate) fn collapse_selected(&mut self) {
    if let Some(selected) = self.selected
      && let Some(entry) = self.entries.get_mut(selected)
    {
      if entry.expanded && !entry.children.is_empty() {
        entry.expanded = false;
      } else if let Some(parent) = entry.parent {
        self.selected = Some(parent);
      }
    }

    self.ensure_selection_visible();
  }

  pub(crate) fn ensure_selection_visible(&mut self) {
    let mut current = self.selected;

    while let Some(idx) = current {
      if self.is_visible(idx) {
        self.selected = Some(idx);
        return;
      }

      current = self.entries.get(idx).and_then(|entry| entry.parent);
    }

    self.selected = self.visible_indexes().first().copied();
  }

  pub(crate) fn expand_selected(&mut self) {
    if let Some(selected) = self.selected
      && let Some(entry) = self.entries.get_mut(selected)
    {
      if entry.children.is_empty() {
        return;
      }

      if entry.expanded {
        if let Some(child) = entry.children.first().copied() {
          self.selected = Some(child);
        }
      } else {
        entry.expanded = true;
      }
    }

    self.ensure_selection_visible();
  }

  pub(crate) fn is_visible(&self, idx: usize) -> bool {
    let mut current = Some(idx);

    while let Some(i) = current {
      if let Some(parent) = self.entries.get(i).and_then(|entry| entry.parent) {
        if let Some(parent_entry) = self.entries.get(parent)
          && !parent_entry.expanded
        {
          return false;
        }

        current = Some(parent);
      } else {
        break;
      }
    }

    true
  }

  pub(crate) fn link(&self) -> &str {
    &self.link
  }

  pub(crate) fn move_by(&mut self, delta: isize) {
    let (visible, selected_pos) = self.visible_with_selection();

    if visible.is_empty() {
      self.selected = None;
      return;
    }

    let current = selected_pos.unwrap_or(0);
    let max_index = visible.len().saturating_sub(1);

    let target = if delta >= 0 {
      let delta_usize = usize::try_from(delta).unwrap_or(usize::MAX);
      current.saturating_add(delta_usize).min(max_index)
    } else {
      let magnitude = delta
        .checked_abs()
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(usize::MAX);

      current.saturating_sub(magnitude)
    };

    self.selected = Some(visible[target]);
  }

  pub(crate) fn new(thread: CommentThread, fallback_link: String) -> Self {
    let CommentThread {
      focus, roots, url, ..
    } = thread;

    let mut entries = Vec::new();
    let mut selected = None;

    for comment in roots {
      Self::push_comment(&mut entries, comment, None, 0, focus, &mut selected);
    }

    if selected.is_none() && !entries.is_empty() {
      selected = Some(0);
    }

    Self {
      entries,
      link: url.unwrap_or(fallback_link),
      offset: 0,
      selected,
    }
  }

  pub(crate) fn page_down(&mut self, amount: usize) {
    let step = amount.saturating_sub(1).max(1);
    let delta = isize::try_from(step).unwrap_or(isize::MAX);
    self.move_by(delta);
  }

  pub(crate) fn page_up(&mut self, amount: usize) {
    let step = amount.saturating_sub(1).max(1);
    let delta = isize::try_from(step).unwrap_or(isize::MAX);
    self.move_by(-delta);
  }

  fn push_comment(
    entries: &mut Vec<CommentEntry>,
    comment: Comment,
    parent: Option<usize>,
    depth: usize,
    focus: Option<u64>,
    selected: &mut Option<usize>,
  ) -> usize {
    let Comment {
      author,
      children,
      dead,
      deleted,
      id,
      text,
    } = comment;

    let body = if deleted {
      "[deleted]".to_string()
    } else if dead {
      "[dead]".to_string()
    } else {
      text.unwrap_or_default()
    };

    let idx = entries.len();

    entries.push(CommentEntry {
      author,
      body,
      children: Vec::new(),
      dead,
      deleted,
      depth,
      expanded: true,
      parent,
    });

    if selected.is_none() && focus == Some(id) {
      *selected = Some(idx);
    }

    let mut child_indices = Vec::new();

    for child in children {
      let child_idx = Self::push_comment(
        entries,
        child,
        Some(idx),
        depth.saturating_add(1),
        focus,
        selected,
      );

      child_indices.push(child_idx);
    }

    if let Some(entry) = entries.get_mut(idx) {
      entry.children = child_indices;
    }

    idx
  }

  pub(crate) fn select_index_at(&mut self, pos: usize) {
    let (visible, _) = self.visible_with_selection();

    if visible.is_empty() {
      self.selected = None;
      return;
    }

    let index = pos.min(visible.len().saturating_sub(1));

    self.selected = Some(visible[index]);
  }

  pub(crate) fn select_next(&mut self) {
    let (visible, selected_pos) = self.visible_with_selection();

    if visible.is_empty() {
      self.selected = None;
      return;
    }

    let current = selected_pos.unwrap_or(0);
    let next = (current + 1).min(visible.len().saturating_sub(1));

    self.selected = Some(visible[next]);
  }

  pub(crate) fn select_previous(&mut self) {
    let (visible, selected_pos) = self.visible_with_selection();

    if visible.is_empty() {
      self.selected = None;
      return;
    }

    let current = selected_pos.unwrap_or(0);
    let previous = current.saturating_sub(1);

    self.selected = Some(visible[previous]);
  }

  pub(crate) fn toggle_selected(&mut self) {
    if let Some(selected) = self.selected
      && let Some(entry) = self.entries.get_mut(selected)
    {
      if entry.children.is_empty() {
        return;
      }

      entry.expanded = !entry.expanded;
    }

    self.ensure_selection_visible();
  }

  pub(crate) fn visible_indexes(&self) -> Vec<usize> {
    let mut visible = Vec::new();

    for idx in 0..self.entries.len() {
      if self.is_visible(idx) {
        visible.push(idx);
      }
    }

    visible
  }

  pub(crate) fn visible_with_selection(&self) -> (Vec<usize>, Option<usize>) {
    let visible = self.visible_indexes();

    let selected_pos = self
      .selected
      .and_then(|selected| visible.iter().position(|&idx| idx == selected));

    (visible, selected_pos)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn make_comment(id: u64, children: Vec<Comment>) -> Comment {
    Comment {
      author: Some(format!("user{id}")),
      children,
      dead: false,
      deleted: false,
      id,
      text: Some(format!("comment {id}")),
    }
  }

  fn make_view(focus: Option<u64>) -> CommentView {
    let child = make_comment(2, Vec::new());

    let parent = make_comment(1, vec![child]);

    CommentView::new(
      CommentThread {
        focus,
        roots: vec![parent],
        url: None,
      },
      "fallback".to_string(),
    )
  }

  #[test]
  fn new_selects_focused_comment_when_present() {
    let view = make_view(Some(2));
    assert_eq!(view.selected, Some(1));
    assert_eq!(view.link(), "fallback");
  }

  #[test]
  fn toggle_selected_collapses_and_expands_comments() {
    let mut view = make_view(None);
    assert!(view.entries[0].expanded);

    view.toggle_selected();
    assert!(!view.entries[0].expanded);

    view.toggle_selected();
    assert!(view.entries[0].expanded);
  }

  #[test]
  fn collapse_selected_moves_to_parent_when_child_selected() {
    let mut view = make_view(None);
    view.select_index_at(1);
    assert_eq!(view.selected, Some(1));
    view.collapse_selected();
    assert_eq!(view.selected, Some(0));
  }

  #[test]
  fn expand_selected_moves_into_first_child() {
    let mut view = make_view(None);
    view.expand_selected();
    assert_eq!(view.selected, Some(1));
  }

  #[test]
  fn ensure_selection_visible_promotes_hidden_selection() {
    let mut view = make_view(None);
    view.select_index_at(1);
    view.entries[0].expanded = false;
    view.ensure_selection_visible();
    assert_eq!(view.selected, Some(0));
  }

  #[test]
  fn visible_indexes_respect_collapsed_ancestors() {
    let mut view = make_view(None);
    assert_eq!(view.visible_indexes(), vec![0, 1]);
    view.entries[0].expanded = false;
    assert_eq!(view.visible_indexes(), vec![0]);
  }
}

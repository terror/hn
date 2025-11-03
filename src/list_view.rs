pub(crate) struct ListView<T> {
  items: Vec<T>,
  offset: usize,
  selected: usize,
}

impl<T> Default for ListView<T> {
  fn default() -> Self {
    Self {
      items: Vec::new(),
      offset: 0,
      selected: 0,
    }
  }
}

impl<T> ListView<T> {
  pub(crate) fn extend<I>(&mut self, items: I)
  where
    I: IntoIterator<Item = T>,
  {
    self.items.extend(items);
  }

  pub(crate) fn is_empty(&self) -> bool {
    self.items.is_empty()
  }

  pub(crate) fn items(&self) -> &[T] {
    &self.items
  }

  pub(crate) fn len(&self) -> usize {
    self.items.len()
  }

  pub(crate) fn new(items: Vec<T>) -> Self {
    Self {
      items,
      offset: 0,
      selected: 0,
    }
  }

  pub(crate) fn offset(&self) -> usize {
    let selected = self.selected_index().unwrap_or(0);

    if self.items.is_empty() {
      0
    } else {
      self.offset.min(selected)
    }
  }

  pub(crate) fn selected_index(&self) -> Option<usize> {
    if self.items.is_empty() {
      None
    } else {
      Some(self.selected.min(self.items.len().saturating_sub(1)))
    }
  }

  pub(crate) fn selected_item(&self) -> Option<&T> {
    self
      .selected_index()
      .and_then(|index| self.items.get(index))
  }

  pub(crate) fn selected_raw(&self) -> usize {
    self.selected
  }

  pub(crate) fn set_offset(&mut self, offset: usize) {
    if self.items.is_empty() {
      self.offset = 0;
    } else {
      let max_offset = self.items.len().saturating_sub(1);
      self.offset = offset.min(max_offset);
    }
  }

  pub(crate) fn set_selected(&mut self, index: usize) {
    if self.items.is_empty() {
      self.selected = 0;
    } else {
      self.selected = index.min(self.items.len().saturating_sub(1));
    }
  }
}

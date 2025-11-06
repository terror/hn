use super::*;

pub(crate) struct State {
  active_tab: usize,
  bookmarks: Bookmarks,
  bookmarks_tab_index: Option<usize>,
  help: HelpView,
  list_height: usize,
  message: String,
  mode: Mode,
  next_request_id: u64,
  pending_comment: Option<PendingComment>,
  pending_effects: Vec<Effect>,
  pending_search: Option<PendingSearch>,
  pending_selections: Vec<Option<usize>>,
  search_input: Option<SearchInput>,
  search_tab_index: Option<usize>,
  tab_loading: Vec<bool>,
  tab_views: Vec<Option<ListView<ListEntry>>>,
  tabs: Vec<Tab>,
  transient_message: Option<TransientMessage>,
}

impl State {
  fn cancel_search(&mut self) {
    if let Some(input) = self.search_input.take() {
      self.message = input.message_backup;
    }
  }

  pub(crate) fn clear_pending_effects(&mut self) {
    self.pending_effects.clear();
  }

  fn close_comments(&mut self) {
    self.restore_active_list_view();

    if !self.help.is_visible() {
      self.message = LIST_STATUS.into();
    }
  }

  fn current_entry(&self) -> Option<&ListEntry> {
    self
      .list_view(self.active_tab)
      .and_then(|view| view.selected_item())
  }

  pub(crate) fn dispatch_command(
    &mut self,
    command: Command,
  ) -> Result<CommandDispatch> {
    debug_assert!(
      self.pending_effects.is_empty(),
      "command dispatch should start without pending effects"
    );

    let mut should_exit = false;

    match command {
      Command::Quit => {
        should_exit = true;
      }
      Command::ShowHelp => self.help.show(&mut self.message),
      Command::HideHelp => self.help.hide(&mut self.message),
      Command::StartSearch => self.start_search(),
      Command::CancelSearch => self.cancel_search(),
      Command::SubmitSearch => self.submit_search()?,
      Command::SwitchTabLeft => self.switch_tab_left(),
      Command::SwitchTabRight => self.switch_tab_right(),
      Command::SelectNext => self.select_next()?,
      Command::SelectPrevious => self.select_previous()?,
      Command::PageDown => self.page_down()?,
      Command::PageUp => self.page_up()?,
      Command::SelectFirst => self.select_index(0)?,
      Command::OpenComments => self.open_comments()?,
      Command::OpenCurrentInBrowser => self.open_current_in_browser(),
      Command::OpenCommentLink => self.open_comment_link(),
      Command::CloseComments => self.close_comments(),
      Command::ToggleBookmark => self.toggle_bookmark()?,
      Command::None => {}
    }

    Ok(CommandDispatch {
      effects: std::mem::take(&mut self.pending_effects),
      should_exit,
    })
  }

  fn ensure_bookmarks_tab(&mut self) -> usize {
    if let Some(index) = self.bookmarks_tab_index {
      return index;
    }

    let entries = self.bookmarks.entries_vec();

    let tab_index = self.tabs.len();

    let category = Category {
      label: "bookmarks",
      kind: CategoryKind::Bookmarks,
    };

    self.tabs.push(Tab {
      category,
      has_more: false,
      label: category.label,
    });

    self.tab_views.push(Some(ListView::new(entries)));
    self.tab_loading.push(false);
    self.pending_selections.push(None);
    self.bookmarks_tab_index = Some(tab_index);

    tab_index
  }

  fn ensure_item(&mut self, tab_index: usize, target_index: usize) -> Result {
    let current_len = self
      .list_view(tab_index)
      .map_or(0, ListView::<ListEntry>::len);

    if target_index < current_len {
      return Ok(());
    }

    let Some(tab) = self.tabs.get(tab_index) else {
      return Ok(());
    };

    if !tab.has_more {
      return Ok(());
    }

    if let Some(slot) = self.pending_selections.get_mut(tab_index) {
      *slot = Some(target_index);
    }

    let is_loading = self.tab_loading.get(tab_index).copied().unwrap_or(false);

    if !is_loading {
      self.start_load_for_tab(tab_index)?;
    }

    Ok(())
  }

  fn ensure_search_tab(&mut self) -> usize {
    if let Some(index) = self.search_tab_index {
      return index;
    }

    let tab_index = self.tabs.len();

    self.tabs.push(Tab {
      category: Category {
        label: "search",
        kind: CategoryKind::Search,
      },
      has_more: false,
      label: "search",
    });

    self.tab_views.push(Some(ListView::default()));
    self.tab_loading.push(false);
    self.pending_selections.push(None);
    self.search_tab_index = Some(tab_index);

    tab_index
  }

  pub(crate) fn handle_event(&mut self, event: Event) {
    match event {
      Event::TabItems { tab_index, result } => {
        if let Some(flag) = self.tab_loading.get_mut(tab_index) {
          *flag = false;
        }

        let target = self
          .pending_selections
          .get_mut(tab_index)
          .and_then(Option::take);

        match result {
          Ok(entries) => {
            if let Some(tab) = self.tabs.get_mut(tab_index) {
              tab.has_more = entries.len() >= INITIAL_BATCH_SIZE;
            }

            if let Some(list) = self.list_view_mut(tab_index) {
              if !entries.is_empty() {
                list.extend(entries);
              }

              if let Some(target) = target {
                if target < list.len() {
                  list.set_selected(target);
                } else if !list.is_empty() {
                  list.set_selected(list.len().saturating_sub(1));
                }
              }
            }

            if !self.help.is_visible() {
              self.message = LIST_STATUS.into();
            }
          }
          Err(error) => {
            if !self.help.is_visible() {
              self.set_transient_message(format!(
                "Could not load more entries: {error}"
              ));
            }
          }
        }
      }
      Event::SearchResults { request_id, result } => {
        let Some(pending) = self.pending_search.as_ref() else {
          return;
        };

        if pending.request_id != request_id {
          return;
        }

        let Some(pending) = self.pending_search.take() else {
          return;
        };

        if let Some(flag) = self.tab_loading.get_mut(pending.tab_index) {
          *flag = false;
        }

        match result {
          Ok((entries, has_more)) => {
            if let Some(tab) = self.tabs.get_mut(pending.tab_index) {
              tab.has_more = has_more;
            }

            let mut view = ListView::new(entries);

            let result_count = view.len();

            if !view.is_empty() {
              view.set_selected(0);
            }

            if let Some(list) = self.list_view_mut(pending.tab_index) {
              *list = view;
            } else if let Some(slot) = self.tab_views.get_mut(pending.tab_index)
            {
              *slot = Some(view);
            }

            if !self.help.is_visible() {
              let truncated = truncate(&pending.query, 40);

              self.message = match result_count {
                0 => format!("No results for \"{truncated}\""),
                1 => format!("Found 1 result for \"{truncated}\""),
                _ => {
                  format!("Found {result_count} results for \"{truncated}\"")
                }
              };
            }
          }
          Err(error) => {
            if !self.help.is_visible() {
              self.set_transient_message(format!("Could not search: {error}"));
            }
          }
        }
      }
      Event::Comments { request_id, result } => {
        let Some(pending) = self.pending_comment.as_ref() else {
          return;
        };

        if pending.request_id != request_id {
          return;
        }

        let Some(pending) = self.pending_comment.take() else {
          return;
        };

        match result {
          Ok(thread) => {
            let view = CommentView::new(thread, pending.comment_link);

            self.store_active_list_view();

            self.mode = Mode::Comments(view);

            if !self.help.is_visible() {
              self.message = COMMENTS_STATUS.into();
            }
          }
          Err(error) => {
            if !self.help.is_visible() {
              self.set_transient_message(format!(
                "Could not load comments: {error}"
              ));
            }
          }
        }
      }
    }
  }

  fn handle_search_key(&mut self, key: KeyEvent) -> Command {
    if self.search_input.is_none() {
      return Command::None;
    }

    match key.code {
      KeyCode::Esc => Command::CancelSearch,
      KeyCode::Enter => Command::SubmitSearch,
      KeyCode::Backspace => {
        if let Some(input) = self.search_input.as_mut() {
          input.buffer.pop();
        }

        self.update_search_message();

        Command::None
      }
      KeyCode::Char(ch) => {
        let modifiers = key.modifiers;

        if modifiers.contains(KeyModifiers::CONTROL)
          || modifiers.contains(KeyModifiers::ALT)
          || modifiers.contains(KeyModifiers::SUPER)
        {
          return Command::None;
        }

        if let Some(input) = self.search_input.as_mut() {
          input.buffer.push(ch);
        }

        self.update_search_message();

        Command::None
      }
      _ => Command::None,
    }
  }

  pub(crate) fn help(&self) -> &HelpView {
    &self.help
  }

  pub(crate) fn help_is_visible(&self) -> bool {
    self.help.is_visible()
  }

  pub(crate) fn list_height(&self) -> usize {
    self.list_height
  }

  fn list_view(&self, index: usize) -> Option<&ListView<ListEntry>> {
    if index >= self.tabs.len() {
      return None;
    }

    if let Mode::List(view) = &self.mode
      && index == self.active_tab
    {
      return Some(view);
    }

    self.tab_views.get(index).and_then(|slot| slot.as_ref())
  }

  fn list_view_mut(
    &mut self,
    index: usize,
  ) -> Option<&mut ListView<ListEntry>> {
    if index >= self.tabs.len() {
      return None;
    }

    match &mut self.mode {
      Mode::List(view) if index == self.active_tab => Some(view),
      _ => self.tab_views.get_mut(index).and_then(|slot| slot.as_mut()),
    }
  }

  pub(crate) fn message(&self) -> &str {
    &self.message
  }

  pub(crate) fn mode_mut(&mut self) -> &mut Mode {
    &mut self.mode
  }

  pub(crate) fn new(
    tabs: Vec<(Tab, ListView<ListEntry>)>,
    bookmarks: Bookmarks,
  ) -> Self {
    let (mut tab_views, mut tab_meta) = (Vec::new(), Vec::new());

    for (tab, view) in tabs {
      tab_meta.push(tab);
      tab_views.push(Some(view));
    }

    let initial_view = tab_views
      .get_mut(0)
      .and_then(Option::take)
      .unwrap_or_default();

    let tab_count = tab_meta.len();

    let tab_loading = vec![false; tab_count];
    let pending_selections = vec![None; tab_count];

    let mut state = Self {
      active_tab: 0,
      bookmarks,
      bookmarks_tab_index: None,
      help: HelpView::new(),
      list_height: 0,
      message: LIST_STATUS.into(),
      mode: Mode::List(initial_view),
      next_request_id: 0,
      pending_comment: None,
      pending_effects: Vec::new(),
      pending_search: None,
      pending_selections,
      search_input: None,
      search_tab_index: None,
      tab_loading,
      tab_views,
      tabs: tab_meta,
      transient_message: None,
    };

    if !state.bookmarks.is_empty() {
      let index = state.ensure_bookmarks_tab();
      state.refresh_bookmarks_view(index);
    }

    state
  }

  fn open_comment_link(&mut self) {
    if let Mode::Comments(view) = &self.mode {
      let url = view
        .selected_comment_link()
        .unwrap_or_else(|| view.link().to_string());

      self.pending_effects.push(Effect::OpenUrl { url });
    }
  }

  fn open_comments(&mut self) -> Result {
    let Some(entry) = self.current_entry() else {
      return Ok(());
    };

    let entry_id = entry.id.clone();

    let id = match entry_id.parse::<u64>() {
      Ok(id) => id,
      Err(error) => {
        self.set_transient_message(format!("Could not load comments: {error}"));
        return Ok(());
      }
    };

    if !self.help.is_visible() {
      self.message = LOADING_COMMENTS_STATUS.into();
    }

    let comment_link =
      format!("https://news.ycombinator.com/item?id={entry_id}");

    let request_id = self.next_request_id;

    self.next_request_id = self.next_request_id.wrapping_add(1);

    self.pending_comment = Some(PendingComment {
      comment_link,
      request_id,
    });

    self.pending_effects.push(Effect::FetchComments {
      item_id: id,
      request_id,
    });

    Ok(())
  }

  fn open_current_in_browser(&mut self) {
    if let Some(entry) = self.current_entry() {
      self.pending_effects.push(Effect::OpenUrl {
        url: entry.resolved_url(),
      });
    }
  }

  fn page_down(&mut self) -> Result {
    if self.tabs.is_empty() {
      return Ok(());
    }

    let tab_index = self.active_tab.min(self.tabs.len().saturating_sub(1));

    let current = self
      .list_view(tab_index)
      .map_or(0, ListView::<ListEntry>::selected_raw);

    let jump = self.page_jump();

    self.select_index(current.saturating_add(jump))
  }

  fn page_jump(&self) -> usize {
    self.list_height.saturating_sub(1).max(1)
  }

  fn page_up(&mut self) -> Result {
    if self.tabs.is_empty() {
      return Ok(());
    }

    let tab_index = self.active_tab.min(self.tabs.len().saturating_sub(1));

    let current = self
      .list_view(tab_index)
      .map_or(0, ListView::<ListEntry>::selected_raw);

    let jump = self.page_jump();

    self.select_index(current.saturating_sub(jump))
  }

  fn refresh_bookmarks_view(&mut self, tab_index: usize) {
    let entries = self.bookmarks.entries_vec();

    if let Some(view) = self.list_view_mut(tab_index) {
      let selected = view.selected_index().unwrap_or(0);
      let offset = view.offset();

      *view = ListView::new(entries);

      if !view.is_empty() {
        let last_index = view.len().saturating_sub(1);
        view.set_selected(selected.min(last_index));
        view.set_offset(offset.min(last_index));
      }
    } else if let Some(slot) = self.tab_views.get_mut(tab_index) {
      let mut view = ListView::new(entries);

      if let Some(existing) = slot.as_ref() {
        let selected = existing.selected_index().unwrap_or(0);
        let offset = existing.offset();

        if !view.is_empty() {
          let last_index = view.len().saturating_sub(1);
          view.set_selected(selected.min(last_index));
          view.set_offset(offset.min(last_index));
        }
      }

      *slot = Some(view);
    }
  }

  fn remove_bookmarks_tab(&mut self) {
    let Some(index) = self.bookmarks_tab_index.take() else {
      return;
    };

    if self.active_tab == index {
      self.mode = Mode::List(ListView::default());
    } else if self.active_tab > index {
      self.active_tab = self.active_tab.saturating_sub(1);
    }

    if let Some(search_index) = self.search_tab_index {
      if search_index == index {
        self.search_tab_index = None;
      } else if search_index > index {
        self.search_tab_index = Some(search_index.saturating_sub(1));
      }
    }

    if index < self.tabs.len() {
      self.tabs.remove(index);
    }

    if index < self.tab_views.len() {
      self.tab_views.remove(index);
    }

    if index < self.tab_loading.len() {
      self.tab_loading.remove(index);
    }

    if index < self.pending_selections.len() {
      self.pending_selections.remove(index);
    }

    if !self.tabs.is_empty() {
      self.active_tab = self.active_tab.min(self.tabs.len().saturating_sub(1));
      self.restore_active_list_view();
    }
  }

  pub(crate) fn resolved_active_tab(&self) -> Option<usize> {
    if self.tabs.is_empty() {
      None
    } else {
      Some(self.active_tab.min(self.tabs.len().saturating_sub(1)))
    }
  }

  fn restore_active_list_view(&mut self) {
    if let Some(slot) = self.tab_views.get_mut(self.active_tab) {
      if let Some(view) = slot.take() {
        self.mode = Mode::List(view);
      } else if !matches!(self.mode, Mode::List(_)) {
        self.mode = Mode::List(ListView::default());
      }
    } else if !matches!(self.mode, Mode::List(_)) {
      self.mode = Mode::List(ListView::default());
    }
  }

  pub(crate) fn search_input_command(
    &mut self,
    key: KeyEvent,
  ) -> Option<Command> {
    if self.search_input.is_some() {
      Some(self.handle_search_key(key))
    } else {
      None
    }
  }

  fn select_index(&mut self, target: usize) -> Result {
    if self.tabs.is_empty() {
      return Ok(());
    }

    let tab_index = self.active_tab.min(self.tabs.len().saturating_sub(1));

    self.ensure_item(tab_index, target)?;

    if let Some(list) = self.list_view_mut(tab_index) {
      if target >= list.len() {
        return Ok(());
      }

      list.set_selected(target);
    }

    Ok(())
  }

  fn select_next(&mut self) -> Result {
    if self.tabs.is_empty() {
      return Ok(());
    }

    let tab_index = self.active_tab.min(self.tabs.len().saturating_sub(1));

    let current = self
      .list_view(tab_index)
      .map_or(0, ListView::<ListEntry>::selected_raw);

    self.select_index(current.saturating_add(1))
  }

  fn select_previous(&mut self) -> Result {
    if self.tabs.is_empty() {
      return Ok(());
    }

    let tab_index = self.active_tab.min(self.tabs.len().saturating_sub(1));

    let current = self
      .list_view(tab_index)
      .map_or(0, ListView::<ListEntry>::selected_raw);

    self.select_index(current.saturating_sub(1))
  }

  pub(crate) fn set_list_height(&mut self, height: usize) {
    self.list_height = height;
  }

  pub(crate) fn set_transient_message(&mut self, message: String) {
    let original = self.transient_message.as_ref().map_or_else(
      || self.message.clone(),
      |transient| transient.original().to_string(),
    );

    self.transient_message =
      Some(TransientMessage::new(message.clone(), original));

    self.message = message;
  }

  fn start_load_for_tab(&mut self, tab_index: usize) -> Result {
    let (category, offset) = if let Some(tab) = self.tabs.get(tab_index) {
      if !tab.has_more {
        return Ok(());
      }

      let offset = self
        .list_view(tab_index)
        .map_or(0, ListView::<ListEntry>::len);

      (tab.category, offset)
    } else {
      return Ok(());
    };

    if let Some(flag) = self.tab_loading.get_mut(tab_index) {
      if *flag {
        return Ok(());
      }

      *flag = true;
    } else {
      return Ok(());
    }

    if !self.help.is_visible() {
      self.message = LOADING_ENTRIES_STATUS.into();
    }

    self.pending_effects.push(Effect::FetchTabItems {
      tab_index,
      category,
      offset,
    });

    Ok(())
  }

  fn start_search(&mut self) {
    if self.search_input.is_some() {
      return;
    }

    let backup = self.message.clone();

    self.search_input = Some(SearchInput::new(backup));

    self.update_search_message();
  }

  fn store_active_list_view(&mut self) {
    if let Mode::List(view) = &mut self.mode
      && let Some(slot) = self.tab_views.get_mut(self.active_tab)
    {
      *slot = Some(std::mem::take(view));
    }
  }

  fn submit_search(&mut self) -> Result {
    let Some(search) = self.search_input.take() else {
      return Ok(());
    };

    let query = search.buffer.trim().to_string();

    if query.is_empty() {
      self.message = search.message_backup;
      return Ok(());
    }

    if matches!(self.mode, Mode::Comments(_)) {
      self.restore_active_list_view();
    }

    let tab_index = self.ensure_search_tab();

    self.store_active_list_view();
    self.active_tab = tab_index;
    self.restore_active_list_view();

    if let Some(list) = self.list_view_mut(tab_index) {
      *list = ListView::default();
    } else if let Some(slot) = self.tab_views.get_mut(tab_index) {
      *slot = Some(ListView::default());
    }

    if let Some(tab) = self.tabs.get_mut(tab_index) {
      tab.has_more = false;
    }

    let request_id = self.next_request_id;

    self.next_request_id = self.next_request_id.wrapping_add(1);

    if let Some(flag) = self.tab_loading.get_mut(tab_index) {
      *flag = true;
    }

    self.pending_search = Some(PendingSearch {
      query: query.clone(),
      request_id,
      tab_index,
    });

    self.message = format!("Searching for \"{}\"...", truncate(&query, 40));

    self
      .pending_effects
      .push(Effect::FetchSearchResults { query, request_id });

    Ok(())
  }

  fn switch_tab_left(&mut self) {
    let tab_count = self.tabs.len();

    if tab_count != 0 {
      self.store_active_list_view();
      self.active_tab = (self.active_tab + tab_count - 1) % tab_count;
      self.restore_active_list_view();
    }
  }

  fn switch_tab_right(&mut self) {
    let tab_count = self.tabs.len();

    if tab_count != 0 {
      self.store_active_list_view();
      self.active_tab = (self.active_tab + 1) % tab_count;
      self.restore_active_list_view();
    }
  }

  fn sync_bookmarks_tab(&mut self) {
    if self.bookmarks.is_empty() {
      self.remove_bookmarks_tab();
    } else {
      let index = self.ensure_bookmarks_tab();
      self.refresh_bookmarks_view(index);
    }
  }

  pub(crate) fn tab(&self, index: usize) -> Option<&Tab> {
    self.tabs.get(index)
  }

  pub(crate) fn tab_loading(&self) -> &[bool] {
    &self.tab_loading
  }

  pub(crate) fn tabs(&self) -> &[Tab] {
    &self.tabs
  }

  fn toggle_bookmark(&mut self) -> Result {
    match &mut self.mode {
      Mode::List(_) => self.toggle_list_bookmark(),
      Mode::Comments(_) => self.toggle_comment_bookmark(),
    }
  }

  fn toggle_comment_bookmark(&mut self) -> Result {
    let Mode::Comments(view) = &mut self.mode else {
      return Ok(());
    };

    let Some(selected) = view.selected_entry() else {
      return Ok(());
    };

    let entry = selected.to_bookmark_entry();

    let added = self.bookmarks.toggle(&entry)?;

    self.sync_bookmarks_tab();

    if !self.help.is_visible() {
      let title = truncate(&entry.title, 40);

      let message = if added {
        format!("Bookmarked \"{title}\"")
      } else {
        format!("Removed bookmark for \"{title}\"")
      };

      self.set_transient_message(message);
    }

    Ok(())
  }

  fn toggle_list_bookmark(&mut self) -> Result {
    let Some(entry) = self.current_entry().cloned() else {
      return Ok(());
    };

    let added = self.bookmarks.toggle(&entry)?;

    self.sync_bookmarks_tab();

    if !self.help.is_visible() {
      let title = truncate(&entry.title, 40);

      let message = if added {
        format!("Bookmarked \"{title}\"")
      } else {
        format!("Removed bookmark for \"{title}\"")
      };

      self.set_transient_message(message);
    }

    Ok(())
  }

  fn update_search_message(&mut self) {
    if let Some(input) = &self.search_input {
      let prompt = input.prompt();
      self.message = truncate(&prompt, 80);
    }
  }

  pub(crate) fn update_transient_message(&mut self) {
    if let Some(transient) = self.transient_message.clone() {
      if self.message != transient.current() {
        self.transient_message = None;
      } else if transient.is_expired() {
        self.message = transient.original().to_string();
        self.transient_message = None;
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn empty_bookmarks() -> Bookmarks {
    let unique = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .expect("system time before UNIX_EPOCH")
      .as_nanos();

    let path =
      std::env::temp_dir().join(format!("hn_app_state_test_{unique}.json"));

    // SAFETY: Scoped test code sets env var to isolate bookmarks file.
    unsafe {
      std::env::set_var("HN_BOOKMARKS_FILE", &path);
    }

    let bookmarks = Bookmarks::load().expect("load bookmarks");

    // SAFETY: Test restores original environment variable state before exit.
    unsafe {
      std::env::remove_var("HN_BOOKMARKS_FILE");
    }

    bookmarks
  }

  fn sample_state_with_entry() -> State {
    let entry = ListEntry {
      detail: None,
      id: "42".to_string(),
      title: "Example".to_string(),
      url: Some("https://example.com".to_string()),
    };

    let view = ListView::new(vec![entry]);

    let tab = Tab {
      category: Category {
        label: "top",
        kind: CategoryKind::Stories("topstories"),
      },
      has_more: false,
      label: "top",
    };

    State::new(vec![(tab, view)], empty_bookmarks())
  }

  #[test]
  fn dispatch_open_comments_emits_fetch_effect() {
    let mut state = sample_state_with_entry();

    let dispatch = state
      .dispatch_command(Command::OpenComments)
      .expect("dispatch succeeds");

    assert!(!dispatch.should_exit);

    assert_eq!(dispatch.effects.len(), 1);

    match &dispatch.effects[0] {
      Effect::FetchComments { item_id, .. } => assert_eq!(*item_id, 42),
      _ => panic!("unexpected effect variant"),
    }

    assert_eq!(state.message, LOADING_COMMENTS_STATUS);
  }

  #[test]
  fn open_comment_link_opens_selected_comment() {
    let mut state = sample_state_with_entry();

    let comment_view = CommentView::new(
      CommentThread {
        focus: None,
        roots: vec![Comment {
          author: Some("user".to_string()),
          children: Vec::new(),
          dead: false,
          deleted: false,
          id: 123,
          text: Some("body".to_string()),
        }],
      },
      "https://news.ycombinator.com/item?id=42".to_string(),
    );

    state.mode = Mode::Comments(comment_view);

    state.open_comment_link();

    assert_eq!(state.pending_effects.len(), 1);

    match &state.pending_effects[0] {
      Effect::OpenUrl { url } => assert_eq!(url, "https://news.ycombinator.com/item?id=123"),
      _ => panic!("unexpected effect variant"),
    }
  }

  #[test]
  fn start_search_sets_search_input() {
    let mut state = sample_state_with_entry();

    let dispatch = state
      .dispatch_command(Command::StartSearch)
      .expect("dispatch succeeds");

    assert!(dispatch.effects.is_empty());
    assert!(!dispatch.should_exit);
    assert!(state.search_input.is_some());

    assert_eq!(state.message, "Search: ");
  }
}

use super::*;

pub(crate) struct App {
  active_tab: usize,
  client: Client,
  event_rx: UnboundedReceiver<Event>,
  event_tx: UnboundedSender<Event>,
  handle: Handle,
  list_height: usize,
  message: String,
  message_backup: Option<String>,
  mode: Mode,
  next_request_id: u64,
  pending_comment: Option<PendingComment>,
  pending_effects: Vec<Effect>,
  pending_selections: Vec<Option<usize>>,
  show_help: bool,
  tab_loading: Vec<bool>,
  tab_views: Vec<Option<ListView<ListEntry>>>,
  tabs: Vec<Tab>,
}

impl App {
  fn close_comments(&mut self) {
    self.restore_active_list_view();

    if !self.show_help {
      self.message = DEFAULT_STATUS.into();
    }
  }

  fn comment_list_item(entry: &CommentEntry, available_width: u16) -> ListItem {
    let depth_indent = "  ".repeat(entry.depth);
    let indent = format!("{BASE_INDENT}{depth_indent}");

    let toggle = entry.has_children().then_some(if entry.expanded {
      "[-]"
    } else {
      "[+]"
    });

    let mut header = vec![Span::raw(indent.clone())];

    if let Some(symbol) = toggle {
      header.push(Span::raw(symbol));
      header.push(Span::raw(" "));
    }

    header.push(Span::styled(
      entry.header(),
      Style::default().fg(Color::White),
    ));

    let mut lines = vec![Line::from(header)];

    if !entry.body().is_empty() {
      let body_indent = indent.clone();
      let prefix_width = body_indent.chars().count();

      let max_width = available_width as usize;
      let wrap_width = max_width.saturating_sub(prefix_width).max(1);

      for line in wrap_text(entry.body(), wrap_width) {
        lines.push(Line::from(vec![
          Span::raw(body_indent.clone()),
          Span::styled(line, Style::default().fg(Color::DarkGray)),
        ]));
      }
    }

    lines.push(Line::from(Span::raw(indent.clone())));

    ListItem::new(lines)
  }

  fn current_entry(&self) -> Option<&ListEntry> {
    self
      .list_view(self.active_tab)
      .and_then(|view| view.selected_item())
  }

  fn dispatch_command(&mut self, command: Command) -> Result<CommandDispatch> {
    debug_assert!(
      self.pending_effects.is_empty(),
      "command dispatch should start without pending effects"
    );

    let mut should_exit = false;

    match command {
      Command::Quit => {
        should_exit = true;
      }
      Command::ShowHelp => self.show_help(),
      Command::HideHelp => self.hide_help(),
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
      Command::None => {}
    }

    Ok(CommandDispatch {
      effects: self.take_pending_effects(),
      should_exit,
    })
  }

  fn draw(&mut self, frame: &mut Frame) {
    let layout = Layout::default()
      .direction(Direction::Vertical)
      .margin(1)
      .constraints([
        Constraint::Length(2),
        Constraint::Min(0),
        Constraint::Length(1),
      ])
      .split(frame.area());

    self.list_height = layout[1].height as usize;

    let tab_titles: Vec<Line> = self
      .tabs
      .iter()
      .map(|tab| Line::from(tab.label.to_uppercase()))
      .collect();

    let tabs = Tabs::new(tab_titles)
      .select(self.active_tab.min(self.tabs.len().saturating_sub(1)))
      .style(Style::default().fg(Color::DarkGray))
      .highlight_style(
        Style::default()
          .fg(Color::Cyan)
          .add_modifier(Modifier::BOLD),
      )
      .divider(Span::raw(" "));

    frame.render_widget(tabs, layout[0]);

    let (list_items, selected_index, offset) = match &mut self.mode {
      Mode::List(view) => {
        let items = view.items();
        let selected_index = view.selected_index();
        let offset = view.offset();

        let list_items: Vec<ListItem> = if items.is_empty() {
          vec![ListItem::new(Line::from(vec![
            Span::raw(BASE_INDENT),
            Span::raw("Nothing to show. Try another tab."),
          ]))]
        } else {
          items
            .iter()
            .map(|entry| {
              let mut lines = vec![Line::from(vec![
                Span::raw(BASE_INDENT),
                Span::styled(
                  entry.title.clone(),
                  Style::default().fg(Color::White),
                ),
              ])];

              if let Some(detail) = &entry.detail {
                lines.push(Line::from(vec![
                  Span::raw(BASE_INDENT),
                  Span::styled(
                    detail.clone(),
                    Style::default().fg(Color::DarkGray),
                  ),
                ]));
              }

              lines.push(Line::from(Span::raw(BASE_INDENT)));

              ListItem::new(lines)
            })
            .collect()
        };

        (list_items, selected_index, offset)
      }
      Mode::Comments(view) => {
        let (visible, selected_pos) = view.visible_with_selection();

        let list_items: Vec<ListItem> = if visible.is_empty() {
          vec![ListItem::new(Line::from(vec![
            Span::raw(BASE_INDENT),
            Span::raw("No comments yet."),
          ]))]
        } else {
          visible
            .iter()
            .map(|&idx| {
              Self::comment_list_item(&view.entries[idx], layout[1].width)
            })
            .collect()
        };

        let offset = view.offset.min(selected_pos.unwrap_or(0));

        (list_items, selected_pos, offset)
      }
    };

    let mut state = ListState::default()
      .with_selected(selected_index)
      .with_offset(offset);

    let list = List::new(list_items)
      .highlight_style(
        Style::default()
          .fg(Color::Cyan)
          .add_modifier(Modifier::BOLD),
      )
      .highlight_symbol("");

    frame.render_stateful_widget(list, layout[1], &mut state);

    match &mut self.mode {
      Mode::List(view) => {
        view.set_offset(state.offset());
      }
      Mode::Comments(view) => {
        view.offset = state.offset();
      }
    }

    let status = Paragraph::new(self.message.clone())
      .style(Style::default().fg(Color::DarkGray));

    frame.render_widget(status, layout[2]);

    if self.show_help {
      let area = Self::help_area(frame.area());

      frame.render_widget(Clear, area);

      let help = Paragraph::new(HELP_TEXT)
        .block(Block::default().title(HELP_TITLE).borders(Borders::ALL))
        .wrap(Wrap { trim: true });

      frame.render_widget(help, area);
    }
  }

  fn enqueue_effect(&mut self, effect: Effect) {
    self.pending_effects.push(effect);
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

  fn execute_effect(&mut self, effect: Effect) {
    match effect {
      Effect::FetchComments {
        item_id,
        request_id,
      } => {
        let (client, sender) = (self.client.clone(), self.event_tx.clone());

        let handle = self.handle.clone();

        handle.spawn(async move {
          let _ = sender.send(Event::CommentsLoaded {
            request_id,
            result: client.fetch_thread(item_id).await,
          });
        });
      }
      Effect::FetchTabItems {
        tab_index,
        category,
        offset,
      } => {
        let (client, sender) = (self.client.clone(), self.event_tx.clone());

        let handle = self.handle.clone();

        handle.spawn(async move {
          let _ = sender.send(Event::TabItemsLoaded {
            tab_index,
            result: client
              .fetch_category_items(category, offset, INITIAL_BATCH_SIZE)
              .await,
          });
        });
      }
      Effect::OpenUrl { url } => match webbrowser::open(&url) {
        Ok(()) => {
          self.message = format!("Opened in browser: {}", truncate(&url, 80));
        }
        Err(error) => {
          self.message = format!("Could not open link: {error}");
        }
      },
    }
  }

  fn handle_help_key(key: KeyEvent) -> Command {
    match key.code {
      KeyCode::Char('?') | KeyCode::Esc => Command::HideHelp,
      KeyCode::Char('q' | 'Q') => Command::Quit,
      _ => Command::None,
    }
  }

  fn help_area(area: Rect) -> Rect {
    fn saturating_usize_to_u16(value: usize) -> u16 {
      u16::try_from(value).unwrap_or(u16::MAX)
    }

    let (line_count, max_line_width) =
      HELP_TEXT
        .lines()
        .fold((0usize, 0usize), |(count, width), line| {
          let updated_count = count.saturating_add(1);
          let line_width = line.chars().count();

          (updated_count, width.max(line_width))
        });

    let desired_width =
      saturating_usize_to_u16(max_line_width.saturating_add(2)).max(1);

    let desired_height =
      saturating_usize_to_u16(line_count.saturating_add(2)).max(1);

    let available_width = area.width.saturating_sub(2).max(1);
    let available_height = area.height.saturating_sub(2).max(1);

    let width = available_width.clamp(1, desired_width).min(area.width);
    let height = available_height.clamp(1, desired_height).min(area.height);

    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;

    Rect::new(x, y, width, height)
  }

  fn hide_help(&mut self) {
    if self.show_help {
      self.show_help = false;

      if let Some(message) = self.message_backup.take() {
        self.message = message;
      } else {
        self.message = DEFAULT_STATUS.into();
      }
    }
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

  pub(crate) fn new(
    client: Client,
    tabs: Vec<(Tab, ListView<ListEntry>)>,
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

    let (event_tx, event_rx) = mpsc::unbounded_channel();

    let tab_count = tab_meta.len();

    let tab_loading = vec![false; tab_count];
    let pending_selections = vec![None; tab_count];

    Self {
      active_tab: 0,
      client,
      event_rx,
      event_tx,
      handle: Handle::current(),
      list_height: 0,
      message: DEFAULT_STATUS.into(),
      message_backup: None,
      mode: Mode::List(initial_view),
      next_request_id: 0,
      pending_comment: None,
      pending_effects: Vec::new(),
      pending_selections,
      show_help: false,
      tab_loading,
      tab_views,
      tabs: tab_meta,
    }
  }

  fn open_comment_link(&mut self) {
    if let Mode::Comments(view) = &self.mode {
      self.enqueue_effect(Effect::OpenUrl {
        url: view.link().to_string(),
      });
    }
  }

  fn open_comments(&mut self) -> Result {
    let Some(entry) = self.current_entry() else {
      return Ok(());
    };

    let entry_id = entry.id.clone();
    let entry_url = entry.url.clone();

    let id = match entry_id.parse::<u64>() {
      Ok(id) => id,
      Err(error) => {
        self.message = format!("Could not load comments: {error}");
        return Ok(());
      }
    };

    if !self.show_help {
      self.message = LOADING_COMMENTS_STATUS.into();
    }

    let fallback_link = entry_url
      .filter(|value| !value.is_empty())
      .unwrap_or_else(|| {
        format!("https://news.ycombinator.com/item?id={entry_id}")
      });

    let request_id = self.next_request_id;

    self.next_request_id = self.next_request_id.wrapping_add(1);

    self.pending_comment = Some(PendingComment {
      fallback_link,
      request_id,
    });

    self.enqueue_effect(Effect::FetchComments {
      item_id: id,
      request_id,
    });

    Ok(())
  }

  fn open_current_in_browser(&mut self) {
    if let Some(entry) = self.current_entry() {
      self.enqueue_effect(Effect::OpenUrl {
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

  fn process_pending_events(&mut self) {
    loop {
      match self.event_rx.try_recv() {
        Ok(Event::TabItemsLoaded { tab_index, result }) => {
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

              if !self.show_help {
                self.message = DEFAULT_STATUS.into();
              }
            }
            Err(error) => {
              if !self.show_help {
                self.message = format!("Could not load more entries: {error}");
              }
            }
          }
        }
        Ok(Event::CommentsLoaded { request_id, result }) => {
          let is_current = self
            .pending_comment
            .as_ref()
            .is_some_and(|pending| pending.request_id == request_id);

          if !is_current {
            continue;
          }

          let Some(pending) = self.pending_comment.take() else {
            continue;
          };

          match result {
            Ok(thread) => {
              let view = CommentView::new(thread, pending.fallback_link);

              self.store_active_list_view();

              self.mode = Mode::Comments(view);

              if !self.show_help {
                self.message = COMMENTS_STATUS.into();
              }
            }
            Err(error) => {
              if !self.show_help {
                self.message = format!("Could not load comments: {error}");
              }
            }
          }
        }
        Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
      }
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

  pub(crate) fn run(
    &mut self,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
  ) -> Result {
    loop {
      self.process_pending_events();

      terminal.draw(|frame| self.draw(frame))?;

      if !crossterm_event::poll(Duration::from_millis(200))? {
        self.process_pending_events();
        continue;
      }

      let CrosstermEvent::Key(key) = crossterm_event::read()? else {
        self.process_pending_events();
        continue;
      };

      if key.kind != KeyEventKind::Press {
        self.process_pending_events();
        continue;
      }

      let command = if self.show_help {
        Self::handle_help_key(key)
      } else {
        self.mode.handle_key(key, self.list_height.max(1))
      };

      match self.dispatch_command(command) {
        Ok(dispatch) => {
          for effect in dispatch.effects {
            self.execute_effect(effect);
          }

          if dispatch.should_exit {
            break;
          }

          self.process_pending_events();
        }
        Err(error) => {
          self.pending_effects.clear();
          self.message = format!("error: {error}");
          self.process_pending_events();
        }
      }
    }

    Ok(())
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

  fn show_help(&mut self) {
    if !self.show_help {
      self.message_backup = Some(self.message.clone());
      self.message = HELP_STATUS.into();
      self.show_help = true;
    }
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

    if !self.show_help {
      self.message = LOADING_STATUS.into();
    }

    self.enqueue_effect(Effect::FetchTabItems {
      tab_index,
      category,
      offset,
    });

    Ok(())
  }

  fn store_active_list_view(&mut self) {
    if let Mode::List(view) = &mut self.mode
      && let Some(slot) = self.tab_views.get_mut(self.active_tab)
    {
      *slot = Some(std::mem::take(view));
    }
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

  fn take_pending_effects(&mut self) -> Vec<Effect> {
    std::mem::take(&mut self.pending_effects)
  }
}

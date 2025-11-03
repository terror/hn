use super::*;

const DEFAULT_STATUS: &str =
  "↑/k up • ↓/j down • enter comments • o open link • q/esc quit • ? help";

const COMMENTS_STATUS: &str =
  "↑/k up • ↓/j down • ←/h collapse • →/l expand • enter toggle • esc back";

const HELP_TITLE: &str = "Help";

const HELP_STATUS: &str = "Press ? or esc to close help";

const LOADING_STATUS: &str = "Loading more entries...";

const LOADING_COMMENTS_STATUS: &str = "Loading comments...";

const BASE_INDENT: &str = " ";

const HELP_TEXT: &str = "\
Navigation:
  ← / h   previous tab
  → / l   next tab
  ↑ / k   move selection up
  ↓ / j   move selection down
  pg↓     page down
  pg↑     page up
  ctrl+d  page down
  ctrl+u  page up
  home    jump to first item
  end     jump to last item

Actions:
  enter   view comments for the selected item
  o       open the selected item in your browser
  q       quit hn
  esc     close help or quit from the list
  scroll  keep going past the end to load more stories
  ?       toggle this help

Comments:
  ↑ / k   move selection up
  ↓ / j   move selection down
  pg↓     page down
  pg↑     page up
  ← / h   collapse or go to parent
  → / l   expand or go to first child
  enter   toggle collapse or expand
  esc     return to the story list
";

pub(crate) struct App {
  active_tab: usize,
  client: Client,
  list_height: usize,
  message: String,
  message_backup: Option<String>,
  mode: Mode,
  show_help: bool,
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
      // Keep body text aligned with the toggle/indent instead of additional padding.
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
          vec![ListItem::new(Line::from(
            "Nothing to show. Try another tab.",
          ))]
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

  fn ensure_item(&mut self, tab_index: usize, target_index: usize) -> Result {
    loop {
      let needs_more = if let Some(tab) = self.tabs.get(tab_index)
        && let Some(list) = self.list_view(tab_index)
      {
        target_index >= list.len() && tab.has_more
      } else {
        false
      };

      if !needs_more {
        return Ok(());
      }

      if !self.load_more_for_tab(tab_index)? {
        return Ok(());
      }
    }
  }

  fn handle_help_key(key: KeyEvent) -> Action {
    match key.code {
      KeyCode::Char('?') | KeyCode::Esc => Action::HideHelp,
      KeyCode::Char('q' | 'Q') => Action::Quit,
      _ => Action::None,
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

  fn load_more_for_tab(&mut self, tab_index: usize) -> Result<bool> {
    let (category, offset) = if let Some(tab) = self.tabs.get(tab_index) {
      if !tab.has_more {
        return Ok(false);
      }

      let offset = self
        .list_view(tab_index)
        .map_or(0, ListView::<ListEntry>::len);

      (tab.category, offset)
    } else {
      return Ok(false);
    };

    let previous_message = if self.show_help {
      None
    } else {
      Some(self.message.clone())
    };

    if previous_message.is_some() {
      self.message = LOADING_STATUS.into();
    }

    let client = self.client.clone();

    let fetched = tokio::task::block_in_place(|| {
      tokio::runtime::Handle::current().block_on(async move {
        client
          .fetch_category_items(category, offset, INITIAL_BATCH_SIZE)
          .await
      })
    })?;

    if let Some(message) = previous_message {
      self.message = message;
    }

    if let Some(tab) = self.tabs.get_mut(tab_index) {
      tab.has_more = fetched.len() >= INITIAL_BATCH_SIZE;
    } else {
      return Ok(false);
    }

    if fetched.is_empty() {
      return Ok(false);
    }

    if let Some(list) = self.list_view_mut(tab_index) {
      list.extend(fetched);
      return Ok(true);
    }

    Ok(false)
  }

  pub(crate) fn new(
    client: Client,
    tabs: Vec<(Tab, ListView<ListEntry>)>,
  ) -> Self {
    let mut tab_views: Vec<Option<ListView<ListEntry>>> = Vec::new();
    let mut tab_meta = Vec::new();

    for (tab, view) in tabs {
      tab_meta.push(tab);
      tab_views.push(Some(view));
    }

    let initial_view = tab_views
      .get_mut(0)
      .and_then(Option::take)
      .unwrap_or_default();

    Self {
      active_tab: 0,
      client,
      list_height: 0,
      message: DEFAULT_STATUS.into(),
      message_backup: None,
      mode: Mode::List(initial_view),
      show_help: false,
      tab_views,
      tabs: tab_meta,
    }
  }

  fn open_comment_link(&mut self) {
    if let Mode::Comments(view) = &self.mode {
      let link = view.link().to_string();

      match webbrowser::open(&link) {
        Ok(()) => {
          self.message = format!("Opened in browser: {}", truncate(&link, 80));
        }
        Err(err) => {
          self.message = format!("Could not open link: {err}");
        }
      }
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
      Err(err) => {
        self.message = format!("Could not load comments: {err}");
        return Ok(());
      }
    };

    if !self.show_help {
      self.message = LOADING_COMMENTS_STATUS.into();
    }

    let client = self.client.clone();

    let thread = tokio::task::block_in_place(|| {
      tokio::runtime::Handle::current()
        .block_on(async move { client.fetch_thread(id).await })
    });

    let thread = match thread {
      Ok(thread) => thread,
      Err(err) => {
        self.message = format!("Could not load comments: {err}");
        return Ok(());
      }
    };

    let fallback_link = entry_url
      .filter(|value| !value.is_empty())
      .unwrap_or_else(|| {
        format!("https://news.ycombinator.com/item?id={entry_id}")
      });

    let view = CommentView::new(thread, fallback_link);

    self.store_active_list_view();

    self.mode = Mode::Comments(view);

    if !self.show_help {
      self.message = COMMENTS_STATUS.into();
    }

    Ok(())
  }

  fn open_current_in_browser(&mut self) {
    if let Some(entry) = self.current_entry() {
      match entry.open() {
        Ok(link) => {
          self.message = format!("Opened in browser: {}", truncate(&link, 80));
        }
        Err(err) => {
          self.message = format!("Could not open selection: {err}");
        }
      }
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

  fn perform_action(&mut self, action: Action) -> Result<bool> {
    match action {
      Action::Quit => Ok(true),
      Action::ShowHelp => {
        self.show_help();
        Ok(false)
      }
      Action::HideHelp => {
        self.hide_help();
        Ok(false)
      }
      Action::SwitchTabLeft => {
        self.switch_tab_left();
        Ok(false)
      }
      Action::SwitchTabRight => {
        self.switch_tab_right();
        Ok(false)
      }
      Action::SelectNext => {
        self.select_next()?;
        Ok(false)
      }
      Action::SelectPrevious => {
        self.select_previous()?;
        Ok(false)
      }
      Action::PageDown => {
        self.page_down()?;
        Ok(false)
      }
      Action::PageUp => {
        self.page_up()?;
        Ok(false)
      }
      Action::SelectFirst => {
        self.select_index(0)?;
        Ok(false)
      }
      Action::OpenComments => {
        self.open_comments()?;
        Ok(false)
      }
      Action::OpenCurrentInBrowser => {
        self.open_current_in_browser();
        Ok(false)
      }
      Action::OpenCommentLink => {
        self.open_comment_link();
        Ok(false)
      }
      Action::CloseComments => {
        self.close_comments();
        Ok(false)
      }
      Action::None => Ok(false),
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
      terminal.draw(|frame| self.draw(frame))?;

      if !event::poll(Duration::from_millis(200))? {
        continue;
      }

      let Event::Key(key) = event::read()? else {
        continue;
      };

      if key.kind != KeyEventKind::Press {
        continue;
      }

      let action = if self.show_help {
        Self::handle_help_key(key)
      } else {
        self.mode.handle_key(key, self.list_height.max(1))
      };

      match self.perform_action(action) {
        Ok(true) => break,
        Ok(false) => {}
        Err(error) => {
          self.message = format!("error: {error}");
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
}

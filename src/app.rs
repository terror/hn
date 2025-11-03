use super::*;

use {
  crate::{
    comment::{Comment, CommentThread},
    utils::wrap_text,
  },
  crossterm::event::KeyEvent,
  std::convert::TryFrom,
};

const DEFAULT_STATUS: &str =
  "↑/k up • ↓/j down • enter comments • o open link • q/esc quit • ? help";

const COMMENTS_STATUS: &str =
  "↑/k up • ↓/j down • ←/h collapse • →/l expand • enter toggle • esc back";

const HELP_TITLE: &str = "Help";

const HELP_STATUS: &str = "Press ? or esc to close help";

const LOADING_STATUS: &str = "Loading more entries...";

const LOADING_COMMENTS_STATUS: &str = "Loading comments...";

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
  tabs: Vec<TabData>,
}

enum Mode {
  Comments(CommentView),
  List,
}

struct CommentView {
  entries: Vec<CommentEntry>,
  link: String,
  offset: usize,
  selected: Option<usize>,
  title: String,
}

struct CommentEntry {
  author: Option<String>,
  body: String,
  children: Vec<usize>,
  dead: bool,
  deleted: bool,
  depth: usize,
  expanded: bool,
  parent: Option<usize>,
}

impl CommentView {
  fn collapse_selected(&mut self) {
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

  fn ensure_selection_visible(&mut self) {
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

  fn expand_selected(&mut self) {
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

  fn is_empty(&self) -> bool {
    self.entries.is_empty()
  }

  fn is_visible(&self, idx: usize) -> bool {
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

  fn link(&self) -> &str {
    &self.link
  }

  fn move_by(&mut self, delta: isize) {
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

  fn new(
    thread: CommentThread,
    fallback_title: String,
    fallback_link: String,
  ) -> Self {
    let CommentThread {
      focus,
      roots,
      title,
      url,
    } = thread;

    let mut entries = Vec::new();
    let mut selected = None;

    for comment in roots {
      Self::push_comment(&mut entries, comment, None, 0, focus, &mut selected);
    }

    if selected.is_none() && !entries.is_empty() {
      selected = Some(0);
    }

    let title = if focus.is_some() || title.trim().is_empty() {
      fallback_title
    } else {
      title
    };

    Self {
      entries,
      link: url.unwrap_or(fallback_link),
      offset: 0,
      selected,
      title,
    }
  }

  fn page_down(&mut self, amount: usize) {
    let step = amount.saturating_sub(1).max(1);
    let delta = isize::try_from(step).unwrap_or(isize::MAX);
    self.move_by(delta);
  }

  fn page_up(&mut self, amount: usize) {
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

  fn select_index_at(&mut self, pos: usize) {
    let (visible, _) = self.visible_with_selection();

    if visible.is_empty() {
      self.selected = None;
      return;
    }

    let index = pos.min(visible.len().saturating_sub(1));

    self.selected = Some(visible[index]);
  }

  fn select_next(&mut self) {
    let (visible, selected_pos) = self.visible_with_selection();

    if visible.is_empty() {
      self.selected = None;
      return;
    }

    let current = selected_pos.unwrap_or(0);
    let next = (current + 1).min(visible.len().saturating_sub(1));

    self.selected = Some(visible[next]);
  }

  fn select_previous(&mut self) {
    let (visible, selected_pos) = self.visible_with_selection();

    if visible.is_empty() {
      self.selected = None;
      return;
    }

    let current = selected_pos.unwrap_or(0);
    let previous = current.saturating_sub(1);

    self.selected = Some(visible[previous]);
  }

  fn title(&self) -> &str {
    &self.title
  }

  fn toggle_selected(&mut self) {
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

  fn visible_indexes(&self) -> Vec<usize> {
    let mut visible = Vec::new();

    for idx in 0..self.entries.len() {
      if self.is_visible(idx) {
        visible.push(idx);
      }
    }

    visible
  }

  fn visible_with_selection(&self) -> (Vec<usize>, Option<usize>) {
    let visible = self.visible_indexes();

    let selected_pos = self
      .selected
      .and_then(|selected| visible.iter().position(|&idx| idx == selected));

    (visible, selected_pos)
  }
}

impl CommentEntry {
  fn body(&self) -> &str {
    self.body.as_str()
  }

  fn has_children(&self) -> bool {
    !self.children.is_empty()
  }

  fn header(&self) -> String {
    let author = self.author.as_deref().unwrap_or("unknown");

    match (self.deleted, self.dead) {
      (true, _) => format!("{author} (deleted)"),
      (_, true) => format!("{author} (dead)"),
      _ => author.to_string(),
    }
  }
}

impl App {
  fn close_comments(&mut self) {
    self.mode = Mode::List;

    if !self.show_help {
      self.message = DEFAULT_STATUS.into();
    }
  }

  fn comment_list_item(
    entry: &CommentEntry,
    is_selected: bool,
    available_width: u16,
  ) -> ListItem {
    let pointer = if is_selected { "▸ " } else { "  " };
    let pointer_blank = " ".repeat(pointer.chars().count());
    let indent = "  ".repeat(entry.depth);

    let toggle = if entry.has_children() {
      if entry.expanded { "[-]" } else { "[+]" }
    } else {
      "   "
    };

    let mut lines = vec![Line::from(vec![
      Span::raw(pointer),
      Span::raw(indent.clone()),
      Span::raw(toggle),
      Span::raw(" "),
      Span::styled(entry.header(), Style::default().fg(Color::White)),
    ])];

    if !entry.body().is_empty() {
      let prefix_width =
        pointer_blank.chars().count() + indent.chars().count() + 4;

      let max_width = available_width as usize;
      let wrap_width = max_width.saturating_sub(prefix_width).max(1);

      for line in wrap_text(entry.body(), wrap_width) {
        lines.push(Line::from(vec![
          Span::raw(pointer_blank.clone()),
          Span::raw(indent.clone()),
          Span::raw("    "),
          Span::styled(line, Style::default().fg(Color::DarkGray)),
        ]));
      }
    }

    lines.push(Line::from(Span::raw(pointer_blank)));

    ListItem::new(lines)
  }

  fn current_entry(&self) -> Option<&Entry> {
    self.tabs.get(self.active_tab).and_then(|tab| {
      if tab.items.is_empty() {
        None
      } else {
        let index = tab.selected.min(tab.items.len().saturating_sub(1));
        tab.items.get(index)
      }
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
      Mode::List => {
        let (items, selected_index, offset): (&[Entry], Option<usize>, usize) =
          if let Some(tab) = self.tabs.get(self.active_tab) {
            if tab.items.is_empty() {
              (&tab.items, None, 0)
            } else {
              let idx = tab.selected.min(tab.items.len() - 1);
              (&tab.items, Some(idx), tab.offset.min(idx))
            }
          } else {
            (&[], None, 0)
          };

        let list_items: Vec<ListItem> = if items.is_empty() {
          vec![ListItem::new(Line::from(
            "Nothing to show. Try another tab.",
          ))]
        } else {
          items
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
              let pointer = if selected_index == Some(idx) {
                "▸ "
              } else {
                "  "
              };

              let pointer_blank = " ".repeat(pointer.chars().count());
              let indent = pointer_blank.clone();

              let mut lines = vec![Line::from(vec![
                Span::raw(pointer),
                Span::styled(
                  entry.title.clone(),
                  Style::default().fg(Color::White),
                ),
              ])];

              if let Some(detail) = &entry.detail {
                lines.push(Line::from(vec![
                  Span::raw(indent.clone()),
                  Span::styled(
                    detail.clone(),
                    Style::default().fg(Color::DarkGray),
                  ),
                ]));
              }

              lines.push(Line::from(Span::raw(indent)));

              ListItem::new(lines)
            })
            .collect()
        };

        (list_items, selected_index, offset)
      }
      Mode::Comments(view) => {
        let (visible, selected_pos) = view.visible_with_selection();

        let list_items: Vec<ListItem> = if visible.is_empty() {
          vec![ListItem::new(Line::from("No comments yet."))]
        } else {
          visible
            .iter()
            .enumerate()
            .map(|(pos, &idx)| {
              let is_selected = selected_pos == Some(pos);
              Self::comment_list_item(
                &view.entries[idx],
                is_selected,
                layout[1].width,
              )
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
      Mode::List => {
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
          tab.offset = state.offset();
        }
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
      let needs_more = if let Some(tab) = self.tabs.get(tab_index) {
        target_index >= tab.items.len() && tab.has_more
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

  fn handle_comment_key(&mut self, key: KeyEvent) -> Result<bool> {
    let modifiers = key.modifiers;
    let page = self.list_height.max(1);

    match key.code {
      KeyCode::Char('q' | 'Q') => Ok(true),
      KeyCode::Esc => {
        self.close_comments();
        Ok(false)
      }
      KeyCode::Char('?') => {
        self.show_help();
        Ok(false)
      }
      KeyCode::Char('o' | 'O') => {
        self.open_comment_link();
        Ok(false)
      }
      _ => {
        if let Mode::Comments(view) = &mut self.mode {
          match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
              view.select_next();
              Ok(false)
            }
            KeyCode::Up | KeyCode::Char('k') => {
              view.select_previous();
              Ok(false)
            }
            KeyCode::PageDown => {
              view.page_down(page);
              Ok(false)
            }
            KeyCode::PageUp => {
              view.page_up(page);
              Ok(false)
            }
            KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => {
              view.page_down(page);
              Ok(false)
            }
            KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => {
              view.page_up(page);
              Ok(false)
            }
            KeyCode::Left | KeyCode::Char('h') => {
              view.collapse_selected();
              Ok(false)
            }
            KeyCode::Right | KeyCode::Char('l') => {
              view.expand_selected();
              Ok(false)
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
              view.toggle_selected();
              Ok(false)
            }
            KeyCode::Home => {
              view.select_index_at(0);
              Ok(false)
            }
            KeyCode::End => {
              let (visible, _) = view.visible_with_selection();

              if !visible.is_empty() {
                view.select_index_at(visible.len().saturating_sub(1));
              }

              Ok(false)
            }
            _ => Ok(false),
          }
        } else {
          Ok(false)
        }
      }
    }
  }

  fn handle_list_key(&mut self, key: KeyEvent) -> Result<bool> {
    let modifiers = key.modifiers;

    match key.code {
      KeyCode::Char('q' | 'Q') | KeyCode::Esc => Ok(true),
      KeyCode::Char('?') => {
        self.show_help();
        Ok(false)
      }
      KeyCode::Left | KeyCode::Char('h') => {
        if !self.tabs.is_empty() {
          self.active_tab =
            (self.active_tab + self.tabs.len() - 1) % self.tabs.len();
        }

        Ok(false)
      }
      KeyCode::Right | KeyCode::Char('l') => {
        if !self.tabs.is_empty() {
          self.active_tab = (self.active_tab + 1) % self.tabs.len();
        }

        Ok(false)
      }
      KeyCode::Down | KeyCode::Char('j') => {
        self.select_next()?;
        Ok(false)
      }
      KeyCode::Up | KeyCode::Char('k') => {
        self.select_previous()?;
        Ok(false)
      }
      KeyCode::PageDown => {
        self.page_down()?;
        Ok(false)
      }
      KeyCode::PageUp => {
        self.page_up()?;
        Ok(false)
      }
      KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => {
        self.page_down()?;
        Ok(false)
      }
      KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => {
        self.page_up()?;
        Ok(false)
      }
      KeyCode::Home => {
        self.select_index(0)?;
        Ok(false)
      }
      KeyCode::End => {
        if let Some(tab) = self.tabs.get_mut(self.active_tab)
          && !tab.items.is_empty()
        {
          tab.selected = tab.items.len() - 1;
        }

        Ok(false)
      }
      KeyCode::Enter => {
        self.open_comments()?;
        Ok(false)
      }
      KeyCode::Char('o' | 'O') => {
        self.open_current_in_browser();
        Ok(false)
      }
      _ => Ok(false),
    }
  }

  fn help_area(area: Rect) -> Rect {
    let max_width = area.width.max(1);
    let max_height = area.height.max(1);

    let width = area.width.saturating_sub(4).clamp(1, max_width.min(68));
    let height = area.height.saturating_sub(4).clamp(1, max_height.min(18));

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

  fn load_more_for_tab(&mut self, tab_index: usize) -> Result<bool> {
    let (category, offset) = if let Some(tab) = self.tabs.get(tab_index) {
      if !tab.has_more {
        return Ok(false);
      }

      (tab.category, tab.items.len())
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
          .fetch_category_items(category, offset, INITIAL_BATCH)
          .await
      })
    })?;

    if let Some(message) = previous_message {
      self.message = message;
    }

    if let Some(tab) = self.tabs.get_mut(tab_index) {
      if fetched.is_empty() {
        tab.has_more = false;
        return Ok(false);
      }

      if fetched.len() < INITIAL_BATCH {
        tab.has_more = false;
      }

      tab.items.extend(fetched);

      return Ok(true);
    }

    Ok(false)
  }

  pub(crate) fn new(client: Client, tabs: Vec<TabData>) -> Self {
    Self {
      active_tab: 0,
      client,
      list_height: 0,
      message: DEFAULT_STATUS.into(),
      message_backup: None,
      mode: Mode::List,
      show_help: false,
      tabs,
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
    let entry_title = entry.title.clone();
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

    let view = CommentView::new(thread, entry_title.clone(), fallback_link);

    self.mode = Mode::Comments(view);

    if !self.show_help
      && let Mode::Comments(view) = &self.mode
    {
      let title_snippet = truncate(view.title(), 40);

      let prefix = if view.is_empty() {
        format!("No comments yet for {title_snippet}")
      } else {
        format!("Comments for {title_snippet}")
      };

      self.message = format!("{prefix} — {COMMENTS_STATUS}");
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

    let current = {
      let tab = &self.tabs[tab_index];
      tab.selected
    };

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

    let current = {
      let tab = &self.tabs[tab_index];
      tab.selected
    };

    let jump = self.page_jump();

    self.select_index(current.saturating_sub(jump))
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

      let action: Result<bool> = if self.show_help {
        match key.code {
          KeyCode::Char('?') | KeyCode::Esc => {
            self.hide_help();
            Ok(false)
          }
          KeyCode::Char('q' | 'Q') => Ok(true),
          _ => Ok(false),
        }
      } else if matches!(self.mode, Mode::List) {
        self.handle_list_key(key)
      } else {
        self.handle_comment_key(key)
      };

      match action {
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

    if let Some(tab) = self.tabs.get_mut(tab_index) {
      if tab.items.is_empty() {
        tab.selected = 0;
      } else {
        tab.selected = target.min(tab.items.len().saturating_sub(1));
      }
    }

    Ok(())
  }

  fn select_next(&mut self) -> Result {
    if self.tabs.is_empty() {
      return Ok(());
    }

    let tab_index = self.active_tab.min(self.tabs.len().saturating_sub(1));

    let current = {
      let tab = &self.tabs[tab_index];
      tab.selected
    };

    self.select_index(current.saturating_add(1))
  }

  fn select_previous(&mut self) -> Result {
    if self.tabs.is_empty() {
      return Ok(());
    }

    let tab_index = self.active_tab.min(self.tabs.len().saturating_sub(1));

    let current = {
      let tab = &self.tabs[tab_index];
      tab.selected
    };

    self.select_index(current.saturating_sub(1))
  }

  fn show_help(&mut self) {
    if !self.show_help {
      self.message_backup = Some(self.message.clone());
      self.message = HELP_STATUS.into();
      self.show_help = true;
    }
  }
}

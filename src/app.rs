use super::*;

const DEFAULT_STATUS: &str =
  "↑/k up • ↓/j down • enter open • q/esc quit • ? help";

const HELP_TITLE: &str = "Help";

const HELP_STATUS: &str = "Press ? or esc to close help";

const LOADING_STATUS: &str = "Loading more entries...";

const HELP_TEXT: &str = "\
Navigation:
  ← / h  previous tab
  → / l  next tab
  ↑ / k  move selection up
  ↓ / j  move selection down
  pg↓     page down
  pg↑     page up
  ctrl+d  page down
  ctrl+u  page up
  home    jump to first item
  end     jump to last item

Actions:
  enter   open the selected item in your browser
  q       quit hn
  esc     close help or quit from the list
  scroll  keep going past the end to load more stories
  ?       toggle this help
";

pub(crate) struct App {
  active_tab: usize,
  client: Client,
  list_height: usize,
  message: String,
  message_backup: Option<String>,
  show_help: bool,
  tabs: Vec<TabData>,
}

impl App {
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

    let (items, selected_index): (&[Entry], Option<usize>) =
      if let Some(tab) = self.tabs.get(self.active_tab) {
        let idx = if tab.items.is_empty() {
          None
        } else {
          Some(tab.selected.min(tab.items.len() - 1))
        };
        (&tab.items, idx)
      } else {
        (&[], None)
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

          lines.push(Line::from(Span::raw(indent.clone())));

          ListItem::new(lines)
        })
        .collect()
    };

    let mut state = ListState::default();

    if let Some(selected) = selected_index {
      state.select(Some(selected));
    }

    let list = List::new(list_items)
      .highlight_style(
        Style::default()
          .fg(Color::Cyan)
          .add_modifier(Modifier::BOLD),
      )
      .highlight_symbol("");

    frame.render_stateful_widget(list, layout[1], &mut state);

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
      show_help: false,
      tabs,
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
      } else {
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
            if let Some(tab) = self.tabs.get(self.active_tab)
              && let Some(entry) = tab.items.get(tab.selected)
            {
              match open_entry(entry) {
                Ok(link) => {
                  self.message =
                    format!("Opened in browser: {}", truncate(&link, 80));
                }
                Err(err) => {
                  self.message = format!("Could not open selection: {err}");
                }
              }
            }

            Ok(false)
          }
          _ => Ok(false),
        }
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

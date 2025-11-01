use super::*;

pub(crate) struct App {
  active_tab: usize,
  message: String,
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
  }

  pub(crate) fn new(tabs: Vec<TabData>) -> Self {
    Self {
      active_tab: 0,
      message: "↑/k up • ↓/j down • enter open • q/esc quit • ? more".into(),
      tabs,
    }
  }

  pub(crate) fn run(
    &mut self,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
  ) -> Result {
    loop {
      terminal.draw(|frame| self.draw(frame))?;

      if event::poll(Duration::from_millis(200))? {
        match event::read()? {
          Event::Key(key) if key.kind == KeyEventKind::Press => {
            match key.code {
              KeyCode::Char('q' | 'Q') | KeyCode::Esc => break,
              KeyCode::Left | KeyCode::Char('h') => {
                if !self.tabs.is_empty() {
                  self.active_tab =
                    (self.active_tab + self.tabs.len() - 1) % self.tabs.len();
                }
              }
              KeyCode::Right | KeyCode::Char('l') => {
                if !self.tabs.is_empty() {
                  self.active_tab = (self.active_tab + 1) % self.tabs.len();
                }
              }
              KeyCode::Down | KeyCode::Char('j') => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab)
                  && !tab.items.is_empty()
                  && tab.selected + 1 < tab.items.len()
                {
                  tab.selected += 1;
                }
              }
              KeyCode::Up | KeyCode::Char('k') => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab)
                  && tab.selected > 0
                {
                  tab.selected -= 1;
                }
              }
              KeyCode::Home => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                  tab.selected = 0;
                }
              }
              KeyCode::End => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab)
                  && !tab.items.is_empty()
                {
                  tab.selected = tab.items.len() - 1;
                }
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
              }
              _ => {}
            }
          }
          _ => {}
        }
      }
    }

    Ok(())
  }
}

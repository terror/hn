use super::*;

pub(crate) struct App {
  client: Client,
  event_rx: UnboundedReceiver<Event>,
  event_tx: UnboundedSender<Event>,
  handle: Handle,
  state: State,
}

impl App {
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

    self.state.set_list_height(layout[1].height as usize);

    let tabs = self.state.tabs();
    let active_tab = self.state.resolved_active_tab().unwrap_or(0);

    let tab_titles: Vec<Line> = tabs
      .iter()
      .map(|tab| Line::from(tab.label.to_uppercase()))
      .collect();

    let tabs_widget = Tabs::new(tab_titles)
      .select(active_tab)
      .style(Style::default().fg(Color::DarkGray))
      .highlight_style(
        Style::default()
          .fg(Color::Cyan)
          .add_modifier(Modifier::BOLD),
      )
      .divider(Span::raw(" "));

    frame.render_widget(tabs_widget, layout[0]);

    let is_loading = self
      .state
      .tab_loading()
      .get(active_tab)
      .copied()
      .unwrap_or(false);

    let is_search_tab = self
      .state
      .tab(active_tab)
      .is_some_and(|tab| matches!(tab.category.kind, CategoryKind::Search));

    let (list_items, selected_index, offset) = match self.state.mode_mut() {
      Mode::List(view) => {
        let items = view.items();
        let selected_index = view.selected_index();
        let offset = view.offset();

        let list_items: Vec<ListItem> = if items.is_empty() {
          let text = if is_loading {
            if is_search_tab {
              LOADING_SEARCH_STATUS
            } else {
              LOADING_ENTRIES_STATUS
            }
          } else if is_search_tab {
            "No results yet. Try another query."
          } else {
            "Nothing to show. Try another tab."
          };

          vec![ListItem::new(Line::from(vec![
            Span::raw(BASE_INDENT),
            Span::raw(text),
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

    let mut list_state = ListState::default()
      .with_selected(selected_index)
      .with_offset(offset);

    let list = List::new(list_items)
      .highlight_style(
        Style::default()
          .fg(Color::Cyan)
          .add_modifier(Modifier::BOLD),
      )
      .highlight_symbol("");

    frame.render_stateful_widget(list, layout[1], &mut list_state);

    self.state.mode_mut().set_offset(list_state.offset());

    let status = Paragraph::new(self.state.message().to_string())
      .style(Style::default().fg(Color::DarkGray));

    frame.render_widget(status, layout[2]);

    self.state.help().draw(frame);
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
          let _ = sender.send(Event::Comments {
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
          let _ = sender.send(Event::TabItems {
            tab_index,
            result: client
              .fetch_category_items(category, offset, INITIAL_BATCH_SIZE)
              .await,
          });
        });
      }
      Effect::FetchSearchResults { query, request_id } => {
        let (client, sender) = (self.client.clone(), self.event_tx.clone());

        let handle = self.handle.clone();

        handle.spawn(async move {
          let _ = sender.send(Event::SearchResults {
            request_id,
            result: client.search_stories(&query, 0, INITIAL_BATCH_SIZE).await,
          });
        });
      }
      Effect::OpenUrl { url } => match webbrowser::open(&url) {
        Ok(()) => {
          self.state.set_transient_message(format!(
            "Opened in browser: {}",
            truncate(&url, 80)
          ));
        }
        Err(error) => {
          self
            .state
            .set_transient_message(format!("Could not open link: {error}"));
        }
      },
    }
  }

  pub(crate) fn new(
    client: Client,
    tabs: Vec<(Tab, ListView<ListEntry>)>,
    bookmarks: Bookmarks,
  ) -> Self {
    let (event_tx, event_rx) = mpsc::unbounded_channel();

    let state = State::new(tabs, bookmarks);

    Self {
      client,
      event_rx,
      event_tx,
      handle: Handle::current(),
      state,
    }
  }

  fn process_pending_events(&mut self) {
    self.state.update_transient_message();

    while let Ok(event) = self.event_rx.try_recv() {
      self.state.handle_event(event);
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

      let command = if self.state.help_is_visible() {
        HelpView::handle_key(key)
      } else if let Some(command) = self.state.search_input_command(key) {
        command
      } else {
        let page = self.state.list_height().max(1);
        self.state.mode_mut().handle_key(key, page)
      };

      match self.state.dispatch_command(command) {
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
          self.state.clear_pending_effects();
          self.state.set_transient_message(format!("error: {error}"));
          self.process_pending_events();
        }
      }
    }

    Ok(())
  }
}

mod category;
mod client;
mod entry;
mod utils;

use {
  category::{Category, CategoryKind},
  client::Client,
  crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{
      EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
      enable_raw_mode,
    },
  },
  entry::Entry,
  futures::{
    future::join_all,
    stream::{self, StreamExt},
  },
  ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph, Tabs},
  },
  serde::{Deserialize, Deserializer},
  serde_json::Value,
  std::{
    io::{self, Stdout},
    time::Duration,
  },
  utils::truncate,
};

const STORY_LIMIT: usize = 30;

#[derive(Debug, Deserialize)]
struct Story {
  by: Option<String>,
  id: u64,
  score: Option<u64>,
  title: String,
  url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CommentResponse {
  hits: Vec<CommentHit>,
}

#[derive(Debug, Deserialize)]
struct CommentHit {
  author: Option<String>,
  comment_text: Option<String>,
  #[serde(rename = "objectID")]
  object_id: String,
  #[serde(deserialize_with = "deserialize_optional_string")]
  story_id: Option<String>,
  story_title: Option<String>,
  story_url: Option<String>,
}

struct TabData {
  items: Vec<Entry>,
  label: &'static str,
  selected: usize,
}

struct App {
  active_tab: usize,
  message: String,
  tabs: Vec<TabData>,
}

type Result<T = (), E = anyhow::Error> = std::result::Result<T, E>;

impl App {
  fn new(tabs: Vec<TabData>) -> Self {
    Self {
      active_tab: 0,
      message: "↑/k up • ↓/j down • enter open • q/esc quit • ? more".into(),
      tabs,
    }
  }
}

fn init_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
  enable_raw_mode()?;

  let mut stdout = io::stdout();
  execute!(stdout, EnterAlternateScreen)?;

  Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn restore_terminal(
  terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> Result {
  disable_raw_mode()?;

  execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

  terminal.show_cursor()?;

  Ok(())
}

fn run(
  terminal: &mut Terminal<CrosstermBackend<Stdout>>,
  mut app: App,
) -> Result {
  loop {
    terminal.draw(|frame| draw(frame, &app))?;

    if event::poll(Duration::from_millis(200))? {
      match event::read()? {
        Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
          KeyCode::Char('q' | 'Q') | KeyCode::Esc => break,
          KeyCode::Left | KeyCode::Char('h') => {
            if !app.tabs.is_empty() {
              app.active_tab =
                (app.active_tab + app.tabs.len() - 1) % app.tabs.len();
            }
          }
          KeyCode::Right | KeyCode::Char('l') => {
            if !app.tabs.is_empty() {
              app.active_tab = (app.active_tab + 1) % app.tabs.len();
            }
          }
          KeyCode::Down | KeyCode::Char('j') => {
            if let Some(tab) = app.tabs.get_mut(app.active_tab)
              && !tab.items.is_empty()
              && tab.selected + 1 < tab.items.len()
            {
              tab.selected += 1;
            }
          }
          KeyCode::Up | KeyCode::Char('k') => {
            if let Some(tab) = app.tabs.get_mut(app.active_tab)
              && tab.selected > 0
            {
              tab.selected -= 1;
            }
          }
          KeyCode::Home => {
            if let Some(tab) = app.tabs.get_mut(app.active_tab) {
              tab.selected = 0;
            }
          }
          KeyCode::End => {
            if let Some(tab) = app.tabs.get_mut(app.active_tab)
              && !tab.items.is_empty()
            {
              tab.selected = tab.items.len() - 1;
            }
          }
          KeyCode::Enter => {
            if let Some(tab) = app.tabs.get(app.active_tab)
              && let Some(entry) = tab.items.get(tab.selected)
            {
              match open_entry(entry) {
                Ok(link) => {
                  app.message =
                    format!("Opened in browser: {}", truncate(&link, 80));
                }
                Err(err) => {
                  app.message = format!("Could not open selection: {err}");
                }
              }
            }
          }
          _ => {}
        },
        _ => {}
      }
    }
  }

  Ok(())
}

fn draw(frame: &mut Frame, app: &App) {
  let layout = Layout::default()
    .direction(Direction::Vertical)
    .margin(1)
    .constraints([
      Constraint::Length(2),
      Constraint::Min(0),
      Constraint::Length(1),
    ])
    .split(frame.area());

  let tab_titles: Vec<Line> = app
    .tabs
    .iter()
    .map(|tab| Line::from(tab.label.to_uppercase()))
    .collect();

  let tabs = Tabs::new(tab_titles)
    .select(app.active_tab.min(app.tabs.len().saturating_sub(1)))
    .style(Style::default().fg(Color::DarkGray))
    .highlight_style(
      Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD),
    )
    .divider(Span::raw(" "));

  frame.render_widget(tabs, layout[0]);

  let (items, selected_index): (&[Entry], Option<usize>) =
    if let Some(tab) = app.tabs.get(app.active_tab) {
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
          Span::styled(entry.title.clone(), Style::default().fg(Color::White)),
        ])];

        if let Some(detail) = &entry.detail {
          lines.push(Line::from(vec![
            Span::raw(indent.clone()),
            Span::styled(detail.clone(), Style::default().fg(Color::DarkGray)),
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

  let status = Paragraph::new(app.message.clone())
    .style(Style::default().fg(Color::DarkGray));

  frame.render_widget(status, layout[2]);
}

fn open_entry(entry: &Entry) -> Result<String, String> {
  let link = entry
    .url
    .clone()
    .filter(|url| !url.is_empty())
    .unwrap_or_else(|| {
      format!("https://news.ycombinator.com/item?id={}", entry.id)
    });

  webbrowser::open(&link)
    .map(|()| link.clone())
    .map_err(|error| error.to_string())
}

fn deserialize_optional_string<'de, D>(
  deserializer: D,
) -> Result<Option<String>, D::Error>
where
  D: Deserializer<'de>,
{
  use serde::de::{self, Unexpected};

  let value = Option::<Value>::deserialize(deserializer)?;

  match value {
    None | Some(serde_json::Value::Null) => Ok(None),
    Some(Value::String(s)) => Ok(Some(s)),
    Some(Value::Number(n)) => Ok(Some(n.to_string())),
    Some(Value::Bool(b)) => Err(de::Error::invalid_type(
      Unexpected::Bool(b),
      &"string or number",
    )),
    Some(Value::Array(_)) => Err(de::Error::invalid_type(
      Unexpected::Seq,
      &"string or number",
    )),
    Some(Value::Object(_)) => Err(de::Error::invalid_type(
      Unexpected::Map,
      &"string or number",
    )),
  }
}

#[tokio::main]
async fn main() -> Result {
  let client = Client::default();

  let tabs = client.load_tabs(STORY_LIMIT).await?;

  let mut terminal = init_terminal()?;

  let result = run(&mut terminal, App::new(tabs));

  restore_terminal(&mut terminal)?;

  result
}

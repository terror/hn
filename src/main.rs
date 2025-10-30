use {
  category::{Category, CategoryKind},
  crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{
      EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
      enable_raw_mode,
    },
  },
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
    error::Error,
    io::{self, Stdout},
    time::Duration,
  },
  webbrowser,
};

mod category;

const API_BASE_URL: &str = "https://hacker-news.firebaseio.com/v0";

const COMMENTS_URL: &str =
  "https://hn.algolia.com/api/v1/search_by_date?tags=comment&hitsPerPage=";

const ITEM_URL: &str = "https://hacker-news.firebaseio.com/v0/item";

const STORY_LIMIT: usize = 30;

const DEFAULT_MESSAGE: &str =
  "Left/Right tabs | Up/Down browse | Enter opens | q quits";

type AppResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Debug, Deserialize)]
struct Story {
  id: u64,
  title: String,
  url: Option<String>,
  by: Option<String>,
  score: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct CommentResponse {
  hits: Vec<CommentHit>,
}

#[derive(Debug, Deserialize)]
struct CommentHit {
  #[serde(rename = "objectID")]
  object_id: String,
  author: Option<String>,
  comment_text: Option<String>,
  story_title: Option<String>,
  #[serde(deserialize_with = "deserialize_optional_string")]
  story_id: Option<String>,
  story_url: Option<String>,
}

struct Entry {
  id: String,
  title: String,
  detail: Option<String>,
  url: Option<String>,
}

struct TabData {
  label: &'static str,
  items: Vec<Entry>,
  selected: usize,
}

struct App {
  tabs: Vec<TabData>,
  active_tab: usize,
  message: String,
}

impl App {
  fn new(tabs: Vec<TabData>) -> Self {
    let message = tabs
      .get(0)
      .map(default_message_for)
      .unwrap_or_else(|| DEFAULT_MESSAGE.to_string());

    Self {
      tabs,
      active_tab: 0,
      message,
    }
  }

  fn refresh_hint(&mut self) {
    if let Some(tab) = self.tabs.get(self.active_tab) {
      self.message = default_message_for(tab);
    } else {
      self.message = DEFAULT_MESSAGE.to_string();
    }
  }
}

async fn load_tabs(limit: usize) -> AppResult<Vec<TabData>> {
  let client = reqwest::Client::new();

  let tasks = Category::all().iter().map(|category| {
    let client = client.clone();

    async move {
      let entries = fetch_category_items(&client, *category, limit).await?;

      Ok::<TabData, Box<dyn Error + Send + Sync>>(TabData {
        label: category.label,
        items: entries,
        selected: 0,
      })
    }
  });

  let results = join_all(tasks).await;

  let mut tabs = Vec::with_capacity(results.len());

  for result in results {
    tabs.push(result?);
  }

  Ok(tabs)
}

async fn fetch_category_items(
  client: &reqwest::Client,
  category: Category,
  limit: usize,
) -> AppResult<Vec<Entry>> {
  let items = match category.kind {
    CategoryKind::Stories(endpoint) => {
      let stories = fetch_stories(client, endpoint, limit).await?;
      stories.into_iter().map(Entry::from_story).collect()
    }
    CategoryKind::Comments => fetch_comments(client, limit).await?,
  };

  Ok(items)
}

async fn fetch_stories(
  client: &reqwest::Client,
  endpoint: &str,
  limit: usize,
) -> AppResult<Vec<Story>> {
  let ids_url = format!("{API_BASE_URL}/{endpoint}.json");
  let story_ids = client.get(ids_url).send().await?.json::<Vec<u64>>().await?;

  let story_ids = story_ids.into_iter().take(limit);

  let responses = stream::iter(story_ids.map(|id| {
    let client = client.clone();
    async move {
      let response = client.get(format!("{ITEM_URL}/{id}.json")).send().await?;
      response.json::<Story>().await
    }
  }))
  .buffered(16)
  .collect::<Vec<_>>()
  .await;

  let mut stories = Vec::with_capacity(responses.len());
  for story in responses {
    stories.push(story?);
  }

  Ok(stories)
}

async fn fetch_comments(
  client: &reqwest::Client,
  limit: usize,
) -> AppResult<Vec<Entry>> {
  let url = format!("{COMMENTS_URL}{limit}");
  let response = client.get(url).send().await?;
  let payload = response.json::<CommentResponse>().await?;

  let entries = payload.hits.into_iter().map(Entry::from_comment).collect();

  Ok(entries)
}

fn init_terminal() -> AppResult<Terminal<CrosstermBackend<Stdout>>> {
  enable_raw_mode()?;

  let mut stdout = io::stdout();
  execute!(stdout, EnterAlternateScreen)?;

  Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn restore_terminal(
  terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> io::Result<()> {
  disable_raw_mode()?;

  execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

  terminal.show_cursor()?;

  Ok(())
}

fn run(
  terminal: &mut Terminal<CrosstermBackend<Stdout>>,
  mut app: App,
) -> AppResult<()> {
  loop {
    terminal.draw(|frame| draw(frame, &app))?;

    if event::poll(Duration::from_millis(200))? {
      match event::read()? {
        Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
          KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => break,
          KeyCode::Left | KeyCode::Char('h') => {
            if !app.tabs.is_empty() {
              app.active_tab =
                (app.active_tab + app.tabs.len() - 1) % app.tabs.len();
              app.refresh_hint();
            }
          }
          KeyCode::Right | KeyCode::Char('l') => {
            if !app.tabs.is_empty() {
              app.active_tab = (app.active_tab + 1) % app.tabs.len();
              app.refresh_hint();
            }
          }
          KeyCode::Down | KeyCode::Char('j') => {
            if let Some(tab) = app.tabs.get_mut(app.active_tab) {
              if !tab.items.is_empty() && tab.selected + 1 < tab.items.len() {
                tab.selected += 1;
              }
            }
            app.refresh_hint();
          }
          KeyCode::Up | KeyCode::Char('k') => {
            if let Some(tab) = app.tabs.get_mut(app.active_tab) {
              if tab.selected > 0 {
                tab.selected -= 1;
              }
            }
            app.refresh_hint();
          }
          KeyCode::Home => {
            if let Some(tab) = app.tabs.get_mut(app.active_tab) {
              tab.selected = 0;
            }
            app.refresh_hint();
          }
          KeyCode::End => {
            if let Some(tab) = app.tabs.get_mut(app.active_tab) {
              if !tab.items.is_empty() {
                tab.selected = tab.items.len() - 1;
              }
            }
            app.refresh_hint();
          }
          KeyCode::Enter => {
            if let Some(tab) = app.tabs.get(app.active_tab) {
              if let Some(entry) = tab.items.get(tab.selected) {
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
          }
          _ => {}
        },
        Event::Resize(_, _) => {}
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
    .map(|_| link.clone())
    .map_err(|error| error.to_string())
}

impl Entry {
  fn from_story(story: Story) -> Self {
    let detail = match (story.score, story.by.as_deref()) {
      (Some(score), Some(by)) => {
        Some(format!("{} by {}", format_points(score), by))
      }
      (Some(score), None) => Some(format_points(score)),
      (None, Some(by)) => Some(format!("by {by}")),
      _ => None,
    };

    Self {
      id: story.id.to_string(),
      title: story.title,
      detail,
      url: story.url,
    }
  }

  fn from_comment(hit: CommentHit) -> Self {
    let author = hit.author.unwrap_or_else(|| "unknown".to_string());

    let snippet = hit
      .comment_text
      .as_deref()
      .map(sanitize_comment)
      .map(|text| text_snippet(&text, 120));

    let detail = snippet.map(|text| format!("{author}: {text}"));

    let title = hit
      .story_title
      .unwrap_or_else(|| "Comment thread".to_string());

    let url = hit.story_url.or_else(|| {
      hit
        .story_id
        .as_ref()
        .map(|id| format!("https://news.ycombinator.com/item?id={id}"))
    });

    Self {
      id: hit.object_id,
      title,
      detail,
      url,
    }
  }
}

fn sanitize_comment(text: &str) -> String {
  let mut cleaned = String::with_capacity(text.len());
  let mut inside_tag = false;
  let mut last_was_space = false;

  for ch in text.chars() {
    match ch {
      '<' => {
        inside_tag = true;
        if !last_was_space {
          cleaned.push(' ');
          last_was_space = true;
        }
      }
      '>' => {
        inside_tag = false;
      }
      _ if inside_tag => {}
      _ if ch.is_whitespace() => {
        if !last_was_space {
          cleaned.push(' ');
          last_was_space = true;
        }
      }
      _ => {
        cleaned.push(ch);
        last_was_space = false;
      }
    }
  }

  let decoded = decode_entities(cleaned.trim());
  decoded.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn decode_entities(input: &str) -> String {
  input
    .replace("&quot;", "\"")
    .replace("&#x27;", "'")
    .replace("&apos;", "'")
    .replace("&lt;", "<")
    .replace("&gt;", ">")
    .replace("&amp;", "&")
}

fn text_snippet(text: &str, max_chars: usize) -> String {
  if text.chars().count() <= max_chars {
    return text.to_string();
  }

  let mut result = String::new();
  for (idx, ch) in text.chars().enumerate() {
    if idx >= max_chars {
      result.push_str("...");
      break;
    }
    result.push(ch);
  }

  result.trim_end().to_string()
}

fn truncate(text: &str, max_chars: usize) -> String {
  if text.chars().count() <= max_chars {
    text.to_string()
  } else {
    let mut out = String::new();
    for (idx, ch) in text.chars().enumerate() {
      if idx >= max_chars {
        out.push_str("...");
        break;
      }
      out.push(ch);
    }
    out
  }
}

fn default_message_for(tab: &TabData) -> String {
  format!(
    "{} tab | {} items | Left/Right tabs | Up/Down browse | Enter opens | q quits",
    tab.label.to_uppercase(),
    tab.items.len()
  )
}

fn format_points(score: u64) -> String {
  match score {
    1 => "1 point".to_string(),
    _ => format!("{score} points"),
  }
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
async fn main() -> AppResult<()> {
  let tabs = load_tabs(STORY_LIMIT).await?;

  let mut terminal = init_terminal()?;

  let result = run(&mut terminal, App::new(tabs));

  restore_terminal(&mut terminal)?;

  result
}

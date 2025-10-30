use {
  crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{
      EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
      enable_raw_mode,
    },
  },
  ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
  },
  serde::Deserialize,
  std::{
    error::Error,
    io::{self, Stdout},
    time::Duration,
  },
  webbrowser,
};

const TOP_STORIES_URL: &str =
  "https://hacker-news.firebaseio.com/v0/topstories.json";
const ITEM_URL: &str = "https://hacker-news.firebaseio.com/v0/item";
const STORY_LIMIT: usize = 30;
const DEFAULT_MESSAGE: &str =
  "Use Up/Down or j/k to navigate • Enter to open • q to quit";

type AppResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Debug, Deserialize, Clone)]
struct Story {
  id: u64,
  title: String,
  url: Option<String>,
  by: Option<String>,
  score: Option<u64>,
}

async fn fetch_top_stories(limit: usize) -> Result<Vec<Story>, reqwest::Error> {
  let client = reqwest::Client::new();

  let story_ids = client
    .get(TOP_STORIES_URL)
    .send()
    .await?
    .json::<Vec<u64>>()
    .await?;

  let mut stories = Vec::new();

  for id in story_ids.into_iter().take(limit) {
    let story = client
      .get(format!("{}/{}.json", ITEM_URL, id))
      .send()
      .await?
      .json::<Story>()
      .await?;

    stories.push(story);
  }

  Ok(stories)
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
  stories: Vec<Story>,
) -> AppResult<()> {
  let mut selected = 0usize;
  let mut message = DEFAULT_MESSAGE.to_string();

  loop {
    terminal.draw(|frame| {
      draw(frame, &stories, selected, &message);
    })?;

    if event::poll(Duration::from_millis(200))? {
      match event::read()? {
        Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
          KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => break,
          KeyCode::Down | KeyCode::Char('j') => {
            if !stories.is_empty() && selected + 1 < stories.len() {
              selected += 1;
            }
            message = DEFAULT_MESSAGE.to_string();
          }
          KeyCode::Up | KeyCode::Char('k') => {
            if selected > 0 {
              selected -= 1;
            }
            message = DEFAULT_MESSAGE.to_string();
          }
          KeyCode::Home => {
            selected = 0;
            message = DEFAULT_MESSAGE.to_string();
          }
          KeyCode::End => {
            if !stories.is_empty() {
              selected = stories.len() - 1;
            }
            message = DEFAULT_MESSAGE.to_string();
          }
          KeyCode::Enter => {
            if let Some(story) = stories.get(selected) {
              match open_story(story) {
                Ok(link) => {
                  message = format!("Opened in browser: {link}");
                }
                Err(err) => {
                  message = format!("Could not open story: {err}");
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

fn draw(frame: &mut Frame, stories: &[Story], selected: usize, message: &str) {
  let chunks = Layout::default()
    .direction(Direction::Vertical)
    .margin(1)
    .constraints([Constraint::Length(3), Constraint::Min(0)])
    .split(frame.area());

  let controls = Paragraph::new(message)
    .block(Block::default().borders(Borders::ALL).title("Controls"));

  frame.render_widget(controls, chunks[0]);

  let items: Vec<ListItem> = if stories.is_empty() {
    vec![ListItem::new(Line::from("No stories available."))]
  } else {
    stories
      .iter()
      .map(|story| {
        let mut lines = vec![Line::from(story.title.clone())];

        if let Some(url) = &story.url {
          lines.push(Line::from(Span::styled(
            url.clone(),
            Style::default().fg(Color::DarkGray),
          )));
        }

        if let (Some(score), Some(by)) = (story.score, story.by.as_deref()) {
          lines.push(Line::from(Span::styled(
            format!("{score} points by {by}"),
            Style::default().fg(Color::Gray),
          )));
        }
        ListItem::new(lines)
      })
      .collect()
  };

  let mut state = ListState::default();

  if !stories.is_empty() {
    state.select(Some(selected.min(stories.len() - 1)));
  }

  let list = List::new(items)
    .block(
      Block::default()
        .borders(Borders::ALL)
        .title(format!("Top Stories ({})", stories.len())),
    )
    .highlight_style(
      Style::default()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("> ");

  frame.render_stateful_widget(list, chunks[1], &mut state);
}

fn open_story(story: &Story) -> Result<String, String> {
  let link = story
    .url
    .clone()
    .filter(|url| !url.is_empty())
    .unwrap_or_else(|| {
      format!("https://news.ycombinator.com/item?id={}", story.id)
    });

  webbrowser::open(&link)
    .map(|_| link.clone())
    .map_err(|error| error.to_string())
}

#[tokio::main]
async fn main() -> AppResult<()> {
  let stories = fetch_top_stories(STORY_LIMIT).await?;

  let mut terminal = init_terminal()?;

  let result = run(&mut terminal, stories);

  restore_terminal(&mut terminal)?;

  result
}

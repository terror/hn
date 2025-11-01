mod app;
mod category;
mod client;
mod entry;
mod story;
mod utils;

use {
  app::App,
  category::{Category, CategoryKind},
  client::Client,
  crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
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
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
      Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap,
    },
  },
  serde::{
    Deserialize, Deserializer,
    de::{self, Unexpected},
  },
  serde_json::Value,
  std::{
    backtrace::BacktraceStatus,
    io::{self, Stdout},
    process,
    time::Duration,
  },
  story::Story,
  utils::{deserialize_optional_string, truncate},
};

const INITIAL_BATCH: usize = 30;

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
  category: Category,
  has_more: bool,
  items: Vec<Entry>,
  label: &'static str,
  selected: usize,
}

type Result<T = (), E = anyhow::Error> = std::result::Result<T, E>;

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

async fn run() -> Result {
  let client = Client::default();

  let tabs = client.load_tabs(INITIAL_BATCH).await?;

  let mut terminal = init_terminal()?;

  let mut app = App::new(client, tabs);

  app.run(&mut terminal)?;

  restore_terminal(&mut terminal)
}

#[tokio::main]
async fn main() {
  if let Err(error) = run().await {
    eprintln!("error: {error}");

    for (i, error) in error.chain().skip(1).enumerate() {
      if i == 0 {
        eprintln!();
        eprintln!("because:");
      }

      eprintln!("- {error}");
    }

    let backtrace = error.backtrace();

    if backtrace.status() == BacktraceStatus::Captured {
      eprintln!("backtrace:");
      eprintln!("{backtrace}");
    }

    process::exit(1);
  }
}

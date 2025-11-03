mod action;
mod app;
mod category;
mod client;
mod comment;
mod comment_entry;
mod comment_hit;
mod comment_response;
mod comment_thread;
mod comment_view;
mod item;
mod list_entry;
mod list_view;
mod mode;
mod story;
mod tab;
mod utils;

use {
  action::Action,
  anyhow::Context,
  app::App,
  category::{Category, CategoryKind},
  client::Client,
  comment::Comment,
  comment_entry::CommentEntry,
  comment_hit::CommentHit,
  comment_response::CommentResponse,
  comment_thread::CommentThread,
  comment_view::CommentView,
  crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
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
  item::Item,
  list_entry::ListEntry,
  list_view::ListView,
  mode::Mode,
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
  tab::Tab,
  utils::{deserialize_optional_string, format_points, truncate, wrap_text},
};

const INITIAL_BATCH_SIZE: usize = 30;

type Result<T = (), E = anyhow::Error> = std::result::Result<T, E>;

fn initialize_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
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

  let tabs = client.load_tabs(INITIAL_BATCH_SIZE).await?;

  let mut terminal = initialize_terminal()?;

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

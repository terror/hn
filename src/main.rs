mod app;
mod category;
mod client;
mod command;
mod command_dispatch;
mod comment;
mod comment_entry;
mod comment_hit;
mod comment_response;
mod comment_thread;
mod comment_view;
mod effect;
mod event;
mod help_view;
mod item;
mod list_entry;
mod list_view;
mod mode;
mod pending_comment;
mod story;
mod tab;
mod utils;

use {
  anyhow::Context,
  app::App,
  category::{Category, CategoryKind},
  client::Client,
  command::Command,
  command_dispatch::CommandDispatch,
  comment::Comment,
  comment_entry::CommentEntry,
  comment_hit::CommentHit,
  comment_response::CommentResponse,
  comment_thread::CommentThread,
  comment_view::CommentView,
  crossterm::{
    event as crossterm_event,
    event::{
      Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
    },
    execute,
    terminal::{
      EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
      enable_raw_mode,
    },
  },
  effect::Effect,
  event::Event,
  futures::{
    future::join_all,
    stream::{self, StreamExt},
  },
  help_view::HelpView,
  item::Item,
  list_entry::ListEntry,
  list_view::ListView,
  mode::Mode,
  pending_comment::PendingComment,
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
    string::String,
    time::Duration,
  },
  story::Story,
  tab::Tab,
  tokio::{
    runtime::Handle,
    sync::mpsc::{
      self, UnboundedReceiver, UnboundedSender, error::TryRecvError,
    },
  },
  utils::{deserialize_optional_string, format_points, truncate, wrap_text},
};

const INITIAL_BATCH_SIZE: usize = 30;

const LIST_STATUS: &str =
  "↑/k up • ↓/j down • enter comments • o open link • q/esc quit • ? help";

const COMMENTS_STATUS: &str =
  "↑/k up • ↓/j down • ←/h collapse • →/l expand • enter toggle • esc back";

const HELP_TITLE: &str = "Help";
const HELP_STATUS: &str = "Press ? or esc to close help";

const LOADING_ENTRIES_STATUS: &str = "Loading more entries...";
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

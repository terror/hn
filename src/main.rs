use {
  anyhow::Context,
  app::App,
  bookmark::Bookmarks,
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
    style::Stylize,
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
  pending_search::PendingSearch,
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
  search_hit::SearchHit,
  search_input::SearchInput,
  search_response::SearchResponse,
  serde::{
    Deserialize, Deserializer,
    de::{self, Unexpected},
  },
  serde_json::Value,
  state::State,
  std::{
    backtrace::BacktraceStatus,
    collections::HashSet,
    env, fs,
    io::{self, IsTerminal, Stdout},
    path::{Path, PathBuf},
    process,
    string::String,
    time::{Duration, Instant},
  },
  story::Story,
  tab::Tab,
  tokio::{
    runtime::Handle,
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
  },
  transient_message::TransientMessage,
  utils::{deserialize_optional_string, format_points, truncate, wrap_text},
};

mod app;
mod bookmark;
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
mod pending_search;
mod search_hit;
mod search_input;
mod search_response;
mod state;
mod story;
mod tab;
mod transient_message;
mod utils;

const INITIAL_BATCH_SIZE: usize = 30;

const LIST_STATUS: &str = "↑/k up • ↓/j down • enter comments • o open link • b bookmark • q/esc quit • ? help";

const COMMENTS_STATUS: &str = "↑/k up • ↓/j down • ←/h collapse • →/l expand • enter toggle • o open comment • b bookmark • esc back";

const HELP_TITLE: &str = "Help";
const HELP_STATUS: &str = "Press ? or esc to close help";

const LOADING_ENTRIES_STATUS: &str = "Loading more entries...";
const LOADING_COMMENTS_STATUS: &str = "Loading comments...";
const LOADING_SEARCH_STATUS: &str = "Searching...";

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
  b       toggle a bookmark for the selected item
  /       start a search (type to edit, enter to submit)
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
  o       open the selected comment in your browser
  b       toggle a bookmark for the selected comment
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

  let bookmarks = Bookmarks::load().context("could not load bookmarks")?;

  let mut terminal = initialize_terminal()?;

  let mut app = App::new(client, tabs, bookmarks);

  app.run(&mut terminal)?;

  restore_terminal(&mut terminal)
}

#[tokio::main]
async fn main() {
  if let Err(error) = run().await {
    let use_color = io::stderr().is_terminal();

    if use_color {
      eprintln!("{} {error}", "error:".bold().red());
    } else {
      eprintln!("error: {error}");
    }

    for (i, error) in error.chain().skip(1).enumerate() {
      if i == 0 {
        eprintln!();

        if use_color {
          eprintln!("{}", "because:".bold().red());
        } else {
          eprintln!("because:");
        }
      }

      if use_color {
        eprintln!("{} {error}", "-".bold().red());
      } else {
        eprintln!("- {error}");
      }
    }

    let backtrace = error.backtrace();

    if backtrace.status() == BacktraceStatus::Captured {
      if use_color {
        eprintln!("{}", "backtrace:".bold().red());
      } else {
        eprintln!("backtrace:");
      }

      eprintln!("{backtrace}");
    }

    process::exit(1);
  }
}

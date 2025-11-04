use super::{command::Command, *};

pub(crate) enum Mode {
  Comments(CommentView),
  List(ListView<ListEntry>),
}

impl Mode {
  pub(crate) fn handle_key(&mut self, key: KeyEvent, page: usize) -> Command {
    match self {
      Mode::List(view) => {
        let modifiers = key.modifiers;

        match key.code {
          KeyCode::Char('q' | 'Q') | KeyCode::Esc => Command::Quit,
          KeyCode::Char('?') => Command::ShowHelp,
          KeyCode::Left | KeyCode::Char('h') => Command::SwitchTabLeft,
          KeyCode::Right | KeyCode::Char('l') => Command::SwitchTabRight,
          KeyCode::Down | KeyCode::Char('j') => Command::SelectNext,
          KeyCode::Up | KeyCode::Char('k') => Command::SelectPrevious,
          KeyCode::PageDown => Command::PageDown,
          KeyCode::PageUp => Command::PageUp,
          KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => {
            Command::PageDown
          }
          KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => {
            Command::PageUp
          }
          KeyCode::Char('/') => Command::StartSearch,
          KeyCode::Char('b' | 'B') => Command::ToggleBookmark,
          KeyCode::Home => Command::SelectFirst,
          KeyCode::End => {
            if !view.is_empty() {
              let last = view.len().saturating_sub(1);
              view.set_selected(last);
            }

            Command::None
          }
          KeyCode::Enter => Command::OpenComments,
          KeyCode::Char('o' | 'O') => Command::OpenCurrentInBrowser,
          _ => Command::None,
        }
      }
      Mode::Comments(view) => {
        let modifiers = key.modifiers;

        match key.code {
          KeyCode::Char('q' | 'Q') => Command::Quit,
          KeyCode::Esc => Command::CloseComments,
          KeyCode::Char('?') => Command::ShowHelp,
          KeyCode::Char('o' | 'O') => Command::OpenCommentLink,
          KeyCode::Down | KeyCode::Char('j') => {
            view.select_next();
            Command::None
          }
          KeyCode::Up | KeyCode::Char('k') => {
            view.select_previous();
            Command::None
          }
          KeyCode::PageDown => {
            view.page_down(page);
            Command::None
          }
          KeyCode::PageUp => {
            view.page_up(page);
            Command::None
          }
          KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => {
            view.page_down(page);
            Command::None
          }
          KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => {
            view.page_up(page);
            Command::None
          }
          KeyCode::Char('/') => Command::StartSearch,
          KeyCode::Left | KeyCode::Char('h') => {
            view.collapse_selected();
            Command::None
          }
          KeyCode::Right | KeyCode::Char('l') => {
            view.expand_selected();
            Command::None
          }
          KeyCode::Enter | KeyCode::Char(' ') => {
            view.toggle_selected();
            Command::None
          }
          KeyCode::Home => {
            view.select_index_at(0);
            Command::None
          }
          KeyCode::Char('b' | 'B') => Command::ToggleBookmark,
          KeyCode::End => {
            let (visible, _) = view.visible_with_selection();

            if !visible.is_empty() {
              view.select_index_at(visible.len().saturating_sub(1));
            }

            Command::None
          }
          _ => Command::None,
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn sample_list_entries() -> Vec<ListEntry> {
    vec![
      ListEntry {
        detail: None,
        id: "1".to_string(),
        title: "First".to_string(),
        url: None,
      },
      ListEntry {
        detail: None,
        id: "2".to_string(),
        title: "Second".to_string(),
        url: None,
      },
    ]
  }

  fn make_list_mode() -> Mode {
    Mode::List(ListView::new(sample_list_entries()))
  }

  fn make_comments_mode() -> Mode {
    Mode::Comments(CommentView::new(
      CommentThread {
        focus: None,
        roots: vec![Comment {
          author: Some("user".to_string()),
          children: Vec::new(),
          dead: false,
          deleted: false,
          id: 1,
          text: Some("body".to_string()),
        }],
        url: None,
      },
      "fallback".to_string(),
    ))
  }

  fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
  }

  #[test]
  fn quitting_from_list_mode_uses_quit_command() {
    assert_eq!(
      make_list_mode().handle_key(key(KeyCode::Char('q')), 0),
      Command::Quit
    );
  }

  #[test]
  fn starting_search_from_comments_mode_returns_command() {
    assert_eq!(
      make_comments_mode().handle_key(key(KeyCode::Char('/')), 0),
      Command::StartSearch
    );
  }

  #[test]
  fn end_key_in_list_mode_selects_last_item() {
    let mut mode = make_list_mode();

    assert_eq!(mode.handle_key(key(KeyCode::End), 0), Command::None);

    if let Mode::List(ref view) = mode {
      assert_eq!(view.selected_index(), Some(1));
    } else {
      panic!("expected list mode");
    }
  }

  #[test]
  fn navigation_keys_in_list_mode_return_expected_commands() {
    let mut mode = make_list_mode();

    let next = mode.handle_key(key(KeyCode::Down), 0);
    assert_eq!(next, Command::SelectNext);

    let prev = mode.handle_key(key(KeyCode::Up), 0);
    assert_eq!(prev, Command::SelectPrevious);
  }
}

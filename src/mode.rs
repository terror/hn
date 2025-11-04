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

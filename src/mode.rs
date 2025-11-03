use super::{action::Action, *};

pub(crate) enum Mode {
  Comments(CommentView),
  List(ListView<ListEntry>),
}

impl Mode {
  pub(crate) fn handle_key(&mut self, key: KeyEvent, page: usize) -> Action {
    match self {
      Mode::List(view) => {
        let modifiers = key.modifiers;

        match key.code {
          KeyCode::Char('q' | 'Q') | KeyCode::Esc => Action::Quit,
          KeyCode::Char('?') => Action::ShowHelp,
          KeyCode::Left | KeyCode::Char('h') => Action::SwitchTabLeft,
          KeyCode::Right | KeyCode::Char('l') => Action::SwitchTabRight,
          KeyCode::Down | KeyCode::Char('j') => Action::SelectNext,
          KeyCode::Up | KeyCode::Char('k') => Action::SelectPrevious,
          KeyCode::PageDown => Action::PageDown,
          KeyCode::PageUp => Action::PageUp,
          KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => {
            Action::PageDown
          }
          KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => {
            Action::PageUp
          }
          KeyCode::Home => Action::SelectFirst,
          KeyCode::End => {
            if !view.is_empty() {
              let last = view.len().saturating_sub(1);
              view.set_selected(last);
            }

            Action::None
          }
          KeyCode::Enter => Action::OpenComments,
          KeyCode::Char('o' | 'O') => Action::OpenCurrentInBrowser,
          _ => Action::None,
        }
      }
      Mode::Comments(view) => {
        let modifiers = key.modifiers;

        match key.code {
          KeyCode::Char('q' | 'Q') => Action::Quit,
          KeyCode::Esc => Action::CloseComments,
          KeyCode::Char('?') => Action::ShowHelp,
          KeyCode::Char('o' | 'O') => Action::OpenCommentLink,
          KeyCode::Down | KeyCode::Char('j') => {
            view.select_next();
            Action::None
          }
          KeyCode::Up | KeyCode::Char('k') => {
            view.select_previous();
            Action::None
          }
          KeyCode::PageDown => {
            view.page_down(page);
            Action::None
          }
          KeyCode::PageUp => {
            view.page_up(page);
            Action::None
          }
          KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => {
            view.page_down(page);
            Action::None
          }
          KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => {
            view.page_up(page);
            Action::None
          }
          KeyCode::Left | KeyCode::Char('h') => {
            view.collapse_selected();
            Action::None
          }
          KeyCode::Right | KeyCode::Char('l') => {
            view.expand_selected();
            Action::None
          }
          KeyCode::Enter | KeyCode::Char(' ') => {
            view.toggle_selected();
            Action::None
          }
          KeyCode::Home => {
            view.select_index_at(0);
            Action::None
          }
          KeyCode::End => {
            let (visible, _) = view.visible_with_selection();

            if !visible.is_empty() {
              view.select_index_at(visible.len().saturating_sub(1));
            }

            Action::None
          }
          _ => Action::None,
        }
      }
    }
  }
}

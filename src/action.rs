#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Action {
  CloseComments,
  HideHelp,
  None,
  OpenCommentLink,
  OpenComments,
  OpenCurrentInBrowser,
  PageDown,
  PageUp,
  Quit,
  SelectFirst,
  SelectNext,
  SelectPrevious,
  ShowHelp,
  SwitchTabLeft,
  SwitchTabRight,
}

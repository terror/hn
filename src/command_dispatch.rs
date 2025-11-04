use super::*;

pub(crate) struct CommandDispatch {
  pub(crate) effects: Vec<Effect>,
  pub(crate) should_exit: bool,
}

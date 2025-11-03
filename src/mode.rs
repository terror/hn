use super::*;

pub(crate) enum Mode {
  Comments(CommentView),
  List(ListView<ListEntry>),
}

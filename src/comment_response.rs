use super::*;

#[derive(Debug, Deserialize)]
pub(crate) struct CommentResponse {
  pub(crate) hits: Vec<CommentHit>,
}

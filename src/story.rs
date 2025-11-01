use super::*;

#[derive(Debug, Deserialize)]
pub(crate) struct Story {
  pub(crate) by: Option<String>,
  pub(crate) id: u64,
  pub(crate) score: Option<u64>,
  pub(crate) title: String,
  pub(crate) url: Option<String>,
}

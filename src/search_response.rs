use super::*;

#[derive(Debug, Deserialize)]
pub(crate) struct SearchResponse {
  pub(crate) hits: Vec<SearchHit>,
  #[serde(rename = "nbPages")]
  pub(crate) nb_pages: usize,
  pub(crate) page: usize,
}

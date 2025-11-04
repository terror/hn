use super::*;

#[derive(Debug, Deserialize)]
pub(crate) struct SearchHit {
  pub(crate) author: Option<String>,
  #[serde(rename = "objectID")]
  pub(crate) object_id: String,
  pub(crate) points: Option<u64>,
  pub(crate) title: Option<String>,
  pub(crate) url: Option<String>,
}

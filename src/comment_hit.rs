use super::*;

#[derive(Debug, Deserialize)]
pub(crate) struct CommentHit {
  pub(crate) author: Option<String>,
  pub(crate) comment_text: Option<String>,
  #[serde(rename = "objectID")]
  pub(crate) object_id: String,
  #[serde(deserialize_with = "deserialize_optional_string")]
  pub(crate) story_id: Option<String>,
  pub(crate) story_title: Option<String>,
  pub(crate) story_url: Option<String>,
}

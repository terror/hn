use super::*;

#[derive(Debug, Deserialize)]
pub(crate) struct Item {
  pub(crate) by: Option<String>,
  pub(crate) dead: Option<bool>,
  pub(crate) deleted: Option<bool>,
  pub(crate) id: u64,
  pub(crate) kids: Option<Vec<u64>>,
  pub(crate) text: Option<String>,
  #[allow(dead_code)]
  pub(crate) title: Option<String>,
  pub(crate) r#type: Option<String>,
  pub(crate) url: Option<String>,
}

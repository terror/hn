use super::*;

pub(crate) fn open_entry(entry: &Entry) -> Result<String, String> {
  let link = entry
    .url
    .clone()
    .filter(|url| !url.is_empty())
    .unwrap_or_else(|| {
      format!("https://news.ycombinator.com/item?id={}", entry.id)
    });

  webbrowser::open(&link)
    .map(|()| link.clone())
    .map_err(|error| error.to_string())
}

pub(crate) fn deserialize_optional_string<'de, D>(
  deserializer: D,
) -> Result<Option<String>, D::Error>
where
  D: Deserializer<'de>,
{
  let value = Option::<Value>::deserialize(deserializer)?;

  match value {
    None | Some(Value::Null) => Ok(None),
    Some(Value::String(s)) => Ok(Some(s)),
    Some(Value::Number(n)) => Ok(Some(n.to_string())),
    Some(Value::Bool(b)) => Err(de::Error::invalid_type(
      Unexpected::Bool(b),
      &"string or number",
    )),
    Some(Value::Array(_)) => Err(de::Error::invalid_type(
      Unexpected::Seq,
      &"string or number",
    )),
    Some(Value::Object(_)) => Err(de::Error::invalid_type(
      Unexpected::Map,
      &"string or number",
    )),
  }
}

pub(crate) fn sanitize_comment(text: &str) -> String {
  let mut cleaned = String::with_capacity(text.len());
  let mut inside_tag = false;
  let mut last_was_space = false;

  for ch in text.chars() {
    match ch {
      '<' => {
        inside_tag = true;

        if !last_was_space {
          cleaned.push(' ');
          last_was_space = true;
        }
      }
      '>' => {
        inside_tag = false;
      }
      _ if inside_tag => {}
      _ if ch.is_whitespace() => {
        if !last_was_space {
          cleaned.push(' ');
          last_was_space = true;
        }
      }
      _ => {
        cleaned.push(ch);
        last_was_space = false;
      }
    }
  }

  let cleaned = cleaned
    .trim()
    .replace("&quot;", "\"")
    .replace("&#x27;", "'")
    .replace("&apos;", "'")
    .replace("&lt;", "<")
    .replace("&gt;", ">")
    .replace("&amp;", "&");

  cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn truncate(text: &str, max_chars: usize) -> String {
  if text.chars().count() <= max_chars {
    return text.to_string();
  }

  let mut result = String::new();

  for (idx, ch) in text.chars().enumerate() {
    if idx >= max_chars {
      result.push_str("...");
      break;
    }

    result.push(ch);
  }

  result.trim_end().to_string()
}

pub(crate) fn format_points(score: u64) -> String {
  match score {
    1 => "1 point".to_string(),
    _ => format!("{score} points"),
  }
}

#[cfg(test)]
mod tests {
  use {super::*, serde::Deserialize};

  #[test]
  fn truncate_returns_original_when_within_limit() {
    assert_eq!(truncate("short", 10), "short");
  }

  #[test]
  fn truncate_appends_ellipsis_when_exceeding_limit() {
    assert_eq!(truncate("This is a longer line", 4), "This...");
  }

  #[test]
  fn truncate_preserves_exact_length_strings() {
    assert_eq!(truncate("exact", 5), "exact");
  }

  #[test]
  fn sanitize_comment_strips_tags_and_decodes_entities() {
    assert_eq!(
      sanitize_comment(
        "<p>Hello &amp; <i>goodbye</i></p>\n<ul><li>First</li><li>Second</li></ul>"
      ),
      "Hello & goodbye First Second"
    );
  }

  #[test]
  fn sanitize_comment_collapses_whitespace() {
    assert_eq!(
      sanitize_comment("<div>Multiple   spaces<br/>and\tlines</div>"),
      "Multiple spaces and lines"
    );
  }

  #[test]
  fn format_points_handles_singular_and_plural() {
    assert_eq!(format_points(1), "1 point");
    assert_eq!(format_points(2), "2 points");
    assert_eq!(format_points(0), "0 points");
  }

  #[derive(Deserialize, Debug, PartialEq)]
  struct OptionalWrapper {
    #[serde(deserialize_with = "deserialize_optional_string")]
    value: Option<String>,
  }

  fn parse_value(input: &str) -> Result<Option<String>, serde_json::Error> {
    serde_json::from_str::<OptionalWrapper>(input).map(|wrapper| wrapper.value)
  }

  #[test]
  fn deserialize_optional_string_supports_string_numbers_and_null() {
    assert_eq!(
      parse_value(r#"{"value": "hello"}"#).unwrap(),
      Some("hello".to_string())
    );

    assert_eq!(
      parse_value(r#"{"value": 42}"#).unwrap(),
      Some("42".to_string())
    );

    assert_eq!(parse_value(r#"{"value": null}"#).unwrap(), None);

    assert!(
      parse_value(r#"{"value": true}"#).is_err(),
      "bools should fail deserialization"
    );
  }
}

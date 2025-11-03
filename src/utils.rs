use super::*;

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

pub(crate) fn format_points(score: u64) -> String {
  match score {
    1 => "1 point".to_string(),
    _ => format!("{score} points"),
  }
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

pub(crate) fn wrap_text(text: &str, width: usize) -> Vec<String> {
  if text.is_empty() || width == 0 {
    return Vec::new();
  }

  let mut lines = Vec::new();

  for raw_line in text.split('\n') {
    if raw_line.is_empty() {
      lines.push(String::new());
      continue;
    }

    if raw_line.trim().is_empty() {
      lines.push(raw_line.to_string());
      continue;
    }

    if raw_line.starts_with(' ') || raw_line.starts_with('\t') {
      lines.push(raw_line.to_string());
      continue;
    }

    let mut current = String::new();
    let mut current_width = 0;

    for word in raw_line.split_whitespace() {
      let word_width = word.chars().count();

      if current.is_empty() {
        current.push_str(word);
        current_width = word_width;
      } else if current_width + 1 + word_width <= width {
        current.push(' ');
        current.push_str(word);
        current_width += 1 + word_width;
      } else {
        lines.push(current);
        current = word.to_string();
        current_width = word_width;
      }
    }

    if !current.is_empty() {
      lines.push(current);
    }
  }

  if lines.is_empty() {
    vec![text.to_string()]
  } else {
    lines
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(serde::Deserialize, Debug, PartialEq)]
  struct OptionalWrapper {
    #[serde(deserialize_with = "deserialize_optional_string")]
    value: Option<String>,
  }

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
  fn wrap_text_returns_empty_for_empty_input() {
    assert_eq!(wrap_text("", 10), Vec::<String>::new());
  }

  #[test]
  fn wrap_text_keeps_whitespace_only_input() {
    assert_eq!(wrap_text("   ", 5), vec!["   ".to_string()]);
  }

  #[test]
  fn wrap_text_wraps_longer_text() {
    assert_eq!(
      wrap_text("hello brave new world", 11),
      vec!["hello brave".to_string(), "new world".to_string()]
    );
  }

  #[test]
  fn wrap_text_respects_explicit_newlines() {
    assert_eq!(
      wrap_text("first line\n\nsecond line", 20),
      vec![
        "first line".to_string(),
        String::new(),
        "second line".to_string(),
      ]
    );
  }

  #[test]
  fn wrap_text_does_not_wrap_when_within_width() {
    assert_eq!(wrap_text("short text", 20), vec!["short text".to_string()]);
  }

  #[test]
  fn format_points_handles_singular_and_plural() {
    assert_eq!(format_points(1), "1 point");
    assert_eq!(format_points(2), "2 points");
    assert_eq!(format_points(0), "0 points");
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

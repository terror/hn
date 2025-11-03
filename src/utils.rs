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

pub(crate) fn sanitize_comment(text: &str) -> String {
  if text.trim().is_empty() {
    return String::new();
  }

  let config = html2text::config::plain_no_decorate()
    .allow_width_overflow()
    .no_link_wrapping()
    .no_table_borders();

  let rendered = config
    .string_from_read(text.as_bytes(), usize::MAX)
    .unwrap_or_else(|_| html_escape::decode_html_entities(text).into_owned());

  normalize_rendered_comment(&rendered)
}

fn normalize_rendered_comment(rendered: &str) -> String {
  let trimmed = rendered.trim_end_matches('\n');

  if trimmed.is_empty() {
    return String::new();
  }

  let mut lines = trimmed.split('\n').map(str::to_string).collect::<Vec<_>>();

  for line in &mut lines {
    let trimmed_start = line.trim_start_matches(' ');

    if let Some(rest) = trimmed_start.strip_prefix("* ") {
      let indent_len = line.len() - trimmed_start.len();
      let mut converted = String::new();
      converted.push_str(&" ".repeat(indent_len));
      converted.push_str("- ");
      converted.push_str(rest);
      *line = converted;
      continue;
    }

    if let Some(rest) = trimmed_start.strip_prefix("*\t") {
      let indent_len = line.len() - trimmed_start.len();
      let mut converted = String::new();
      converted.push_str(&" ".repeat(indent_len));
      converted.push_str("-\t");
      converted.push_str(rest);
      *line = converted;
    }
  }

  while matches!(lines.last(), Some(last) if last.is_empty()) {
    lines.pop();
  }

  lines.join("\n")
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
  fn sanitize_comment_strips_tags_and_decodes_entities() {
    assert_eq!(
      sanitize_comment(
        "<p>Hello &amp; <i>goodbye</i></p>\n<ul><li>First</li><li>Second</li></ul>"
      ),
      "Hello & goodbye\n- First\n- Second"
    );
  }

  #[test]
  fn sanitize_comment_collapses_whitespace() {
    assert_eq!(
      sanitize_comment("<div>Multiple   spaces<br/>and\tlines</div>"),
      "Multiple spaces\nand lines"
    );
  }

  #[test]
  fn sanitize_comment_preserves_preformatted_blocks() {
    let input = r"
<p>we should aim to parse comments like this better, i believe some newlines got stripped?</p>
<pre><code>#define _(e...) ({e;})

#define x(a,e...) _(s x=a;e)

#define $(a,b) if(a)b;else

#define i(n,e) {int $n=n;int i=0;for(;i<$n;++i){e;}}
</code></pre>
<p>&gt;These are all pretty straight forward, with one subtle caveat I only realized from the annotated code. They're all macros to make common operations more compact: wrapping an expression in a block, defining a variable x and using it, conditional statements, and running an expression n times.</p>
<p>This is war crime territory</p>
    ";

    let expected = "we should aim to parse comments like this better, i believe some newlines got stripped?\n\n#define _(e...) ({e;})\n\n#define x(a,e...) _(s x=a;e)\n\n#define $(a,b) if(a)b;else\n\n#define i(n,e) {int $n=n;int i=0;for(;i<$n;++i){e;}}\n\n>These are all pretty straight forward, with one subtle caveat I only realized from the annotated code. They're all macros to make common operations more compact: wrapping an expression in a block, defining a variable x and using it, conditional statements, and running an expression n times.\n\nThis is war crime territory";

    assert_eq!(sanitize_comment(input), expected);
  }

  #[test]
  fn sanitize_comment_decodes_numeric_entities() {
    assert_eq!(
      sanitize_comment("https:&#x2F;&#x2F;example.com&#47;path"),
      "https://example.com/path"
    );
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

use super::*;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ListEntry {
  pub(crate) detail: Option<String>,
  pub(crate) id: String,
  pub(crate) title: String,
  pub(crate) url: Option<String>,
}

impl From<CommentHit> for ListEntry {
  fn from(hit: CommentHit) -> Self {
    let author = hit.author.unwrap_or_else(|| "unknown".to_string());

    let snippet = hit
      .comment_text
      .as_deref()
      .and_then(|html| {
        html2text::from_read(html.as_bytes(), usize::MAX)
          .ok()
          .map(|text| text.trim_end().to_owned())
      })
      .filter(|text| !text.is_empty())
      .map(|text| truncate(&text, 120));

    let detail = snippet.map(|text| format!("{author}: {text}"));

    let title = hit
      .story_title
      .unwrap_or_else(|| "Comment thread".to_string());

    let url = hit.story_url.or_else(|| {
      hit
        .story_id
        .as_ref()
        .map(|id| format!("https://news.ycombinator.com/item?id={id}"))
    });

    Self {
      detail,
      id: hit.object_id,
      title,
      url,
    }
  }
}

impl From<Story> for ListEntry {
  fn from(story: Story) -> Self {
    let detail = match (story.score, story.by.as_deref()) {
      (Some(score), Some(by)) => {
        Some(format!("{} by {}", format_points(score), by))
      }
      (Some(score), None) => Some(format_points(score)),
      (None, Some(by)) => Some(format!("by {by}")),
      _ => None,
    };

    Self {
      detail,
      id: story.id.to_string(),
      title: story.title,
      url: story.url,
    }
  }
}

impl From<SearchHit> for ListEntry {
  fn from(hit: SearchHit) -> Self {
    let detail = match (hit.points, hit.author.as_deref()) {
      (Some(points), Some(author)) => {
        Some(format!("{} by {}", format_points(points), author))
      }
      (Some(points), None) => Some(format_points(points)),
      (None, Some(author)) => Some(format!("by {author}")),
      _ => None,
    };

    let title = hit.title.unwrap_or_else(|| "Untitled".to_string());

    Self {
      detail,
      id: hit.object_id,
      title,
      url: hit.url,
    }
  }
}

impl ListEntry {
  pub(crate) fn resolved_url(&self) -> String {
    self
      .url
      .clone()
      .filter(|url| !url.is_empty())
      .unwrap_or_else(|| {
        format!("https://news.ycombinator.com/item?id={}", self.id)
      })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn from_story_uses_score_and_author_for_detail() {
    let entry = ListEntry::from(Story {
      by: Some("alice".to_string()),
      id: 123,
      score: Some(10),
      title: "Interesting story".to_string(),
      url: Some("https://example.com/story".to_string()),
    });

    assert_eq!(entry.title, "Interesting story");

    assert_eq!(entry.detail.as_deref(), Some("10 points by alice"));

    assert_eq!(entry.url.as_deref(), Some("https://example.com/story"));
  }

  #[test]
  fn resolved_url_falls_back_to_hn_item_page() {
    let entry = ListEntry {
      detail: None,
      id: "456".to_string(),
      title: "Fallback".to_string(),
      url: None,
    };

    assert_eq!(
      entry.resolved_url(),
      "https://news.ycombinator.com/item?id=456"
    );
  }

  #[test]
  fn from_comment_hit_prefers_story_id_link_and_builds_snippet() {
    let entry = ListEntry::from(CommentHit {
      author: Some("bob".to_string()),
      comment_text: Some("Test detail".to_string()),
      object_id: "789".to_string(),
      story_id: Some("42".to_string()),
      story_title: Some("Comment thread".to_string()),
      story_url: None,
    });

    assert_eq!(entry.detail.as_deref(), Some("bob: Test detail"));

    assert_eq!(
      entry.url.as_deref(),
      Some("https://news.ycombinator.com/item?id=42")
    );

    assert_eq!(entry.title, "Comment thread");
  }

  #[test]
  fn from_search_hit_handles_missing_title_and_author() {
    let entry = ListEntry::from(SearchHit {
      author: None,
      object_id: "s1".to_string(),
      points: Some(5),
      title: None,
      url: Some("https://example.com/search".to_string()),
    });

    assert_eq!(entry.title, "Untitled");

    assert_eq!(entry.detail.as_deref(), Some("5 points"));

    assert_eq!(entry.url.as_deref(), Some("https://example.com/search"));
  }
}

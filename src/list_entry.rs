use super::*;

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

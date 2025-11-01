use {
  super::*,
  crate::utils::{format_points, sanitize_comment, truncate},
};

pub(crate) struct Entry {
  pub(crate) detail: Option<String>,
  pub(crate) id: String,
  pub(crate) title: String,
  pub(crate) url: Option<String>,
}

impl From<CommentHit> for Entry {
  fn from(hit: CommentHit) -> Self {
    let author = hit.author.unwrap_or_else(|| "unknown".to_string());

    let snippet = hit
      .comment_text
      .as_deref()
      .map(sanitize_comment)
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

impl From<Story> for Entry {
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

impl Entry {
  pub(crate) fn open(&self) -> Result<String, String> {
    let link = self
      .url
      .clone()
      .filter(|url| !url.is_empty())
      .unwrap_or_else(|| {
        format!("https://news.ycombinator.com/item?id={}", self.id)
      });

    webbrowser::open(&link)
      .map(|()| link.clone())
      .map_err(|error| error.to_string())
  }
}

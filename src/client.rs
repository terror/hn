use {super::*, anyhow::Context};

use crate::comment::{Comment, CommentThread};

use serde::Deserialize;

#[derive(Clone)]
pub(crate) struct Client {
  client: reqwest::Client,
}

impl Default for Client {
  fn default() -> Self {
    Self {
      client: reqwest::Client::new(),
    }
  }
}

impl Client {
  const API_BASE_URL: &str = "https://hacker-news.firebaseio.com/v0";

  const COMMENTS_URL: &str =
    "https://hn.algolia.com/api/v1/search_by_date?tags=comment&hitsPerPage=";

  const ITEM_URL: &str = "https://hacker-news.firebaseio.com/v0/item";

  pub(crate) async fn fetch_category_items(
    &self,
    category: Category,
    offset: usize,
    count: usize,
  ) -> Result<Vec<Entry>> {
    Ok(match category.kind {
      CategoryKind::Stories(endpoint) => self
        .fetch_stories(endpoint, offset, count)
        .await?
        .into_iter()
        .map(Entry::from)
        .collect(),
      CategoryKind::Comments => self.fetch_comments(offset, count).await?,
    })
  }

  pub(crate) async fn fetch_comments(
    &self,
    offset: usize,
    page_size: usize,
  ) -> Result<Vec<Entry>> {
    let page = offset / page_size.max(1);

    Ok(
      self
        .client
        .get(format!("{}{page_size}&page={page}", Self::COMMENTS_URL))
        .send()
        .await?
        .json::<CommentResponse>()
        .await?
        .hits
        .into_iter()
        .map(Entry::from)
        .collect(),
    )
  }

  pub(crate) async fn fetch_stories(
    &self,
    endpoint: &str,
    offset: usize,
    count: usize,
  ) -> Result<Vec<Story>> {
    let ids_url = format!("{}/{endpoint}.json", Self::API_BASE_URL);

    let story_ids = self
      .client
      .get(ids_url)
      .send()
      .await?
      .json::<Vec<u64>>()
      .await?;

    let story_ids = story_ids.into_iter().skip(offset).take(count);

    let responses = stream::iter(story_ids.map(|id| {
      let client = self.clone();

      async move {
        client
          .client
          .get(format!("{}/{id}.json", Self::ITEM_URL))
          .send()
          .await?
          .json::<Story>()
          .await
      }
    }))
    .buffered(16)
    .collect::<Vec<_>>()
    .await;

    let mut stories = Vec::with_capacity(responses.len());

    for story in responses {
      stories.push(story?);
    }

    Ok(stories)
  }

  pub(crate) async fn load_tabs(&self, limit: usize) -> Result<Vec<TabData>> {
    let tasks = Category::all().iter().map(|category| {
      let client = self.clone();

      let category = *category;

      async move {
        let entries = client
          .fetch_category_items(category, 0, limit)
          .await
          .with_context(|| {
            format!("failed to load {} entries", category.label)
          })?;

        Ok(TabData {
          category,
          has_more: entries.len() == limit,
          items: entries,
          label: category.label,
          selected: 0,
          offset: 0,
        })
      }
    });

    let tabs = join_all(tasks)
      .await
      .into_iter()
      .collect::<Result<Vec<_>>>()?;

    Ok(tabs)
  }

  pub(crate) async fn fetch_thread(&self, id: u64) -> Result<CommentThread> {
    let item = self.fetch_item(id).await?;

    match item.r#type.as_deref() {
      Some("comment") => {
        let title = item
          .title
          .clone()
          .unwrap_or_else(|| format!("Comment {}", item.id));

        let comment = self.build_comment_from_item(item).await?;

        Ok(CommentThread {
          focus: Some(comment.id),
          roots: vec![comment],
          title,
          url: None,
        })
      }
      _ => {
        let title = item
          .title
          .clone()
          .unwrap_or_else(|| format!("Item {}", item.id));

        let url = item.url.clone();

        let roots = self
          .fetch_comment_children(item.kids.clone().unwrap_or_default())
          .await?;

        Ok(CommentThread {
          focus: None,
          roots,
          title,
          url,
        })
      }
    }
  }

  async fn fetch_comment_children(
    &self,
    ids: Vec<u64>,
  ) -> Result<Vec<Comment>> {
    let tasks = ids.into_iter().map(|child_id| {
      let client = self.clone();

      async move { client.fetch_comment(child_id).await }
    });

    let results = stream::iter(tasks).buffered(16).collect::<Vec<_>>().await;

    let mut comments = Vec::new();

    for result in results {
      match result? {
        Some(comment) => comments.push(comment),
        None => {}
      }
    }

    Ok(comments)
  }

  async fn fetch_comment(&self, id: u64) -> Result<Option<Comment>> {
    let item = self.fetch_item(id).await?;

    if item.r#type.as_deref() != Some("comment") {
      return Ok(None);
    }

    let comment = self.build_comment_from_item(item).await?;

    Ok(Some(comment))
  }

  async fn build_comment_from_item(&self, item: Item) -> Result<Comment> {
    let children = self
      .fetch_comment_children(item.kids.clone().unwrap_or_default())
      .await?;

    let text = item
      .text
      .as_deref()
      .map(|content| crate::utils::sanitize_comment(content))
      .filter(|content| !content.is_empty());

    Ok(Comment {
      author: item.by,
      children,
      dead: item.dead.unwrap_or(false),
      deleted: item.deleted.unwrap_or(false),
      id: item.id,
      text,
    })
  }

  async fn fetch_item(&self, id: u64) -> Result<Item> {
    Ok(
      self
        .client
        .get(format!("{}/{id}.json", Self::ITEM_URL))
        .send()
        .await?
        .json::<Item>()
        .await?,
    )
  }
}

#[derive(Debug, Deserialize)]
struct Item {
  by: Option<String>,
  dead: Option<bool>,
  deleted: Option<bool>,
  id: u64,
  kids: Option<Vec<u64>>,
  r#type: Option<String>,
  text: Option<String>,
  title: Option<String>,
  url: Option<String>,
}

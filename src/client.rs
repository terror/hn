use super::*;

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
  pub(crate) async fn load_tabs(
    &self,
    limit: usize,
  ) -> AppResult<Vec<TabData>> {
    let tasks = Category::all().iter().map(|category| {
      let client = self.clone();

      let category = *category;

      async move {
        let entries = client.fetch_category_items(category, limit).await?;

        Ok::<TabData, Box<dyn Error + Send + Sync>>(TabData {
          label: category.label,
          items: entries,
          selected: 0,
        })
      }
    });

    let results = join_all(tasks).await;

    let mut tabs = Vec::with_capacity(results.len());

    for result in results {
      tabs.push(result?);
    }

    Ok(tabs)
  }

  pub(crate) async fn fetch_category_items(
    &self,
    category: Category,
    limit: usize,
  ) -> AppResult<Vec<Entry>> {
    Ok(match category.kind {
      CategoryKind::Stories(endpoint) => self
        .fetch_stories(endpoint, limit)
        .await?
        .into_iter()
        .map(Entry::from_story)
        .collect(),
      CategoryKind::Comments => self.fetch_comments(limit).await?,
    })
  }

  pub(crate) async fn fetch_stories(
    &self,
    endpoint: &str,
    limit: usize,
  ) -> AppResult<Vec<Story>> {
    let ids_url = format!("{API_BASE_URL}/{endpoint}.json");

    let story_ids = self
      .client
      .get(ids_url)
      .send()
      .await?
      .json::<Vec<u64>>()
      .await?;

    let story_ids = story_ids.into_iter().take(limit);

    let responses = stream::iter(story_ids.map(|id| {
      let client = self.clone();

      async move {
        client
          .client
          .get(format!("{ITEM_URL}/{id}.json"))
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

  pub(crate) async fn fetch_comments(
    &self,
    limit: usize,
  ) -> AppResult<Vec<Entry>> {
    Ok(
      self
        .client
        .get(format!("{COMMENTS_URL}{limit}"))
        .send()
        .await?
        .json::<CommentResponse>()
        .await?
        .hits
        .into_iter()
        .map(Entry::from_comment)
        .collect(),
    )
  }
}

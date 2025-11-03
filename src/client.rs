use {super::*, anyhow::Context};

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
}

use {reqwest::Error, serde::Deserialize};

#[derive(Deserialize)]
#[allow(unused)]
struct Story {
  title: String,
  url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
  println!("Fetching top Hacker News stories...");

  let top_stories_url = "https://hacker-news.firebaseio.com/v0/topstories.json";

  let story_ids = reqwest::get(top_stories_url)
    .await?
    .json::<Vec<u64>>()
    .await?;

  for id in story_ids.into_iter().take(10) {
    let story_url =
      format!("https://hacker-news.firebaseio.com/v0/item/{}.json", id);

    let story = reqwest::get(&story_url).await?.json::<Story>().await?;

    println!("- {}", story.title);
  }

  Ok(())
}

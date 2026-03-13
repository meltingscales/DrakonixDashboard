use anyhow::Result;

#[derive(Debug, Clone)]
pub struct RssItem {
    pub title: String,
    pub source: String,
    pub published: Option<String>,
    pub link: Option<String>,
}

pub async fn fetch_feeds(feed_urls: &[String]) -> Result<Vec<RssItem>> {
    if feed_urls.is_empty() {
        return Ok(vec![]);
    }

    let client = reqwest::Client::builder()
        .user_agent("DrakonixDashboard/0.1")
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let mut all_items: Vec<RssItem> = Vec::new();

    for url in feed_urls {
        let url = url.trim();
        if url.is_empty() {
            continue;
        }

        match fetch_one_feed(&client, url).await {
            Ok(items) => all_items.extend(items),
            Err(e) => {
                // Don't fail the whole batch if one feed errors
                all_items.push(RssItem {
                    title: format!("[Error loading feed: {}]", e),
                    source: url.to_string(),
                    published: None,
                    link: None,
                });
            }
        }
    }

    Ok(all_items)
}

async fn fetch_one_feed(client: &reqwest::Client, url: &str) -> Result<Vec<RssItem>> {
    let content = client.get(url).send().await?.bytes().await?;
    let feed = feed_rs::parser::parse(content.as_ref())?;

    let source = feed
        .title
        .as_ref()
        .map(|t| t.content.clone())
        .unwrap_or_else(|| url.to_string());

    let items = feed
        .entries
        .into_iter()
        .map(|entry| {
            let title = entry
                .title
                .map(|t| t.content)
                .unwrap_or_else(|| "(no title)".to_string());

            let link = entry.links.into_iter().next().map(|l| l.href);

            let published = entry.published.or(entry.updated).map(|dt| {
                dt.format("%b %d %H:%M").to_string()
            });

            RssItem {
                title,
                source: source.clone(),
                published,
                link,
            }
        })
        .collect();

    Ok(items)
}

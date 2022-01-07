use anyhow::anyhow;
use censor::Censor;
use reqwest::Client;
use scraper::{Html, Selector};

pub async fn get_lyrics(url: &str) -> anyhow::Result<Vec<String>> {
    let res = Client::new().get(url).send().await?.text().await?;
    let document = Html::parse_document(&res);

    let lyrics_selector = Selector::parse(".lyrics, [class*=Lyrics__Container]").unwrap();
    let mut lyrics = vec![];
    document.select(&lyrics_selector).for_each(|elem| {
        elem.text().for_each(|text| {
            lyrics.push(text.to_string());
        });
    });
    if lyrics.is_empty() {
        return Err(anyhow!("Cannot parse lyrics. For some reason"));
    }
    Ok(lyrics)
}

pub fn find_bad_words(lyrics: Vec<String>, censor: &Censor) -> Vec<String> {
    lyrics
        .into_iter()
        .enumerate()
        .filter_map(|(index, line)| {
            let bad_chars = censor.bad_chars(&line, 0, 0);
            if bad_chars.is_empty() {
                None
            } else {
                let line = line
                    .chars()
                    .into_iter()
                    .enumerate()
                    .map(|(i, c)| {
                        if bad_chars.contains(&i) {
                            format!("~{}~", c)
                        } else {
                            c.into()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("");

                Some(format!("`{}:` {}", index + 1, line))
            }
        })
        .collect()
}

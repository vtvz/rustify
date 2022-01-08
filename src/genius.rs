use anyhow::anyhow;
use censor::Censor;
use reqwest::Client;
use scraper::{Html, Selector};
use teloxide::utils::markdown::escape;

pub async fn get_lyrics(url: &str) -> anyhow::Result<Vec<String>> {
    let res = Client::new().get(url).send().await?.text().await?;
    let document = Html::parse_document(&res);

    let lyrics_selector =
        Selector::parse(".lyrics, [class*=Lyrics__Container]").expect("Should be valid");
    let mut lyrics = vec![];
    document.select(&lyrics_selector).for_each(|elem| {
        elem.text().for_each(|text| {
            lyrics.push(text.to_owned());
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
        .map(|line| escape(&line))
        .enumerate()
        .filter_map(|(index, line)| {
            let (line, bad_chars) = line
                .split_whitespace()
                .map(|word| {
                    let bad_chars = censor.bad_chars(word, 0, 0);
                    let word = if bad_chars.is_empty() {
                        word.to_owned()
                    } else {
                        word.chars()
                            .into_iter()
                            .enumerate()
                            .map(|(i, c)| {
                                if bad_chars.contains(&i) {
                                    format!("__{}__", c)
                                } else {
                                    c.into()
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("")
                            .replace("____", "")
                    };
                    (word, bad_chars.len())
                })
                .fold(("".to_owned(), 0), |accum, item| {
                    (format!("{} {}", accum.0, item.0), accum.1 + item.1)
                });

            if bad_chars == 0 {
                None
            } else {
                Some(format!("`{}:` {}", index + 1, line))
            }
        })
        .collect()
}

use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::sync::LazyLock;

use regex::Regex;

// https://github.com/khanhas/spicetify-cli/blob/master/CustomApps/lyrics-plus/Utils.js#L50
static RG_EXTRA_1: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s-\s.*").expect("Should be compilable"));
static RG_EXTRA_2: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[^\pL_]+").expect("Should be compilable"));
// https://github.com/khanhas/spicetify-cli/blob/master/CustomApps/lyrics-plus/Utils.js#L41
static RG_FEAT_1: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)-\s+(feat|with).*").expect("Should be compilable"));
static RG_FEAT_2: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(\(|\[)(feat|with)\.?\s+.*(\)|\])$").expect("Should be compilable")
});

fn remove_extra_info(name: &str) -> String {
    name.replace(&*RG_EXTRA_1, "")
        .replace(&*RG_EXTRA_2, " ")
        .trim()
        .to_owned()
}

fn remove_song_feat(name: &str) -> String {
    name.replace(&*RG_FEAT_1, "")
        .replace(&*RG_FEAT_2, "")
        .trim()
        .to_owned()
}

#[must_use]
pub fn get_track_names(name: &str) -> HashSet<String> {
    let no_extra = remove_extra_info(name);
    let names = vec![
        name.to_owned(),
        no_extra.clone(),
        remove_song_feat(name),
        remove_song_feat(&no_extra),
    ];

    HashSet::from_iter(names)
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default, PartialEq, PartialOrd)]
pub struct SearchResultConfidence {
    title: f64,
    artist: f64,
}

impl SearchResultConfidence {
    #[must_use]
    pub fn new(artist: f64, title: f64) -> Self {
        Self { title, artist }
    }

    #[must_use]
    pub fn confident(&self, threshold: f64) -> bool {
        self.artist >= threshold && self.title >= threshold
    }

    #[must_use]
    pub fn avg(&self) -> f64 {
        f64::midpoint(self.title, self.artist)
    }
}

impl Display for SearchResultConfidence {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.0}", self.avg() * 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_track_names_variations() {
        let test_cases = vec![
            // (input, expected_contains)
            "Song",
            "Song (feat. Artist)",
            "Song (Feat. Artist)",
            "Song [feat. Artist]",
            "Song (with Artist)",
            "Song - feat. Artist",
            "Song - Remix",
        ];

        for input in test_cases {
            let names = get_track_names(input);
            assert!(
                names.contains("Song"),
                "Expected '{}' to contain '{}', but it didn't. Got: {:?}",
                input,
                "Song",
                names
            );

            assert!(
                names.contains(input),
                "Expected '{input}' to contain '{input}', but it didn't. Got: {names:?}",
            );
        }
    }

    #[test]
    fn test_get_track_names_empty_string() {
        let names = get_track_names("");

        // Should still return a set with the empty string
        assert!(!names.is_empty());
    }

    #[test]
    fn test_get_track_names_unicode() {
        let names = get_track_names("歌曲 (feat. 艺术家)");

        assert!(names.contains("歌曲 (feat. 艺术家)"));
        assert!(names.contains("歌曲"));
    }

    #[test]
    fn test_get_track_names_complex() {
        let names = get_track_names("Song Name - Remix (feat. Artist)");

        let expected_variations = vec![
            "Song Name - Remix (feat. Artist)", // original
            "Song Name",                        // no extra info + no feat
            "Song Name - Remix",                // no feat
        ];

        for expected in expected_variations {
            assert!(
                names.contains(expected),
                "Expected set to contain '{expected}', but it didn't. Got: {names:?}",
            );
        }
    }
}

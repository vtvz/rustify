use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::slice::Iter;

use lazy_static::lazy_static;
use regex::Regex;
use rustrict::{Trie, Type, is_whitespace};
use teloxide::utils::html;

lazy_static! {
    static ref TYPE_THRESHOLD: Type = Type::ANY & !Type::SPAM;
    static ref TYPE_CUSTOM: Type = Type::MODERATE & Type::EVASIVE;
    static ref TYPE_TRIGGER: Type = Type::INAPPROPRIATE | Type::EVASIVE;
}

pub struct Manager;

impl Manager {
    pub fn add_word(word: &str) {
        unsafe { Trie::customize_default().set(word, *TYPE_CUSTOM) }
        tracing::debug!(word, "Added custom word");
    }

    pub fn remove_word(word: &str) {
        unsafe { Trie::customize_default().set(word, Type::NONE) }
        tracing::debug!(word, "Removed custom word");
    }

    pub fn check(text: &[&str]) -> CheckResult {
        CheckResult::perform(text)
    }
}

pub struct CheckResult {
    lines: Vec<LineResult>,
    pub typ: TypeWrapper,
}

pub struct Checker;

impl CheckResult {
    pub fn iter(&self) -> Iter<'_, LineResult> {
        self.lines.iter()
    }

    pub fn should_trigger(&self) -> bool {
        self.typ.is(*TYPE_TRIGGER)
    }

    fn extract_bad_chars(line: &str, censored: &str) -> Vec<usize> {
        let bad_chars: Vec<_> = line
            .chars()
            .enumerate()
            .filter_map(|(i, c)| {
                if censored.chars().nth(i) != Some(c) {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();
        bad_chars
    }

    fn perform(text: &[&str]) -> Self {
        let checks: Vec<_> = text
            .iter()
            .enumerate()
            .map(|(index, line)| {
                let line = html::escape(line);

                let (line, censored, typ) = line
                    .split(is_whitespace)
                    .map(|word| {
                        let (censored, typ) = rustrict::Censor::from_str(word)
                            .with_censor_first_character_threshold(*TYPE_THRESHOLD)
                            .with_censor_threshold(*TYPE_THRESHOLD)
                            .censor_and_analyze();
                        (word, censored, typ)
                    })
                    .fold(
                        (String::new(), String::new(), Type::NONE),
                        |(line, line_censored, mut acc_type), (word, censored, typ)| {
                            if typ.isnt(Type::SAFE) {
                                acc_type |= typ;
                            }

                            (
                                format!("{line} {word}"),
                                format!("{line_censored} {censored}"),
                                acc_type,
                            )
                        },
                    );

                if typ.isnt(*TYPE_THRESHOLD) {
                    return LineResult {
                        no: index,
                        typ: Type::SAFE.into(),
                        censored: line.clone(),
                        line,
                        bad_chars: Default::default(),
                    };
                }

                let bad_chars = Self::extract_bad_chars(&line, &censored);

                LineResult {
                    no: index,
                    typ: typ.into(),
                    line,
                    bad_chars,
                    censored,
                }
            })
            .collect();

        let sum_type = checks
            .iter()
            .map(|LineResult { typ, .. }| typ)
            .filter(|typ| typ.is(*TYPE_THRESHOLD))
            .fold(Type::NONE, |acc, typ| acc | **typ);

        Self {
            lines: checks,
            typ: sum_type.into(),
        }
    }

    pub fn get_profine_words(&self) -> HashSet<String> {
        let mut words = HashSet::new();

        self.iter()
            .for_each(|item| words.extend(item.get_profine_words()));

        words
    }
}

#[derive(Default)]
pub struct LineResult {
    pub no: usize,
    pub typ: TypeWrapper,
    pub line: String,
    pub bad_chars: Vec<usize>,
    pub censored: String,
}

impl LineResult {
    pub fn highlighted(&self) -> String {
        self.line
            .chars()
            .enumerate()
            .map(|(i, c)| {
                match (
                    i != 0 && self.bad_chars.contains(&(i - 1)),
                    self.bad_chars.contains(&i),
                    self.bad_chars.contains(&(i + 1)),
                ) {
                    (false, true, false) => format!("<tg-spoiler><u>{c}</u></tg-spoiler>"),
                    (true, true, false) => format!("{c}</u></tg-spoiler>"),
                    (false, true, true) => format!("<tg-spoiler><u>{c}"),
                    _ => c.into(),
                }
            })
            .collect::<Vec<_>>()
            .join("")
    }

    pub fn get_profine_words(&self) -> HashSet<String> {
        lazy_static! {
            static ref RG_WRAPPER: Regex =
                Regex::new(r"<u>(.*?)</u>").expect("Should be compilable");
        }

        let haystack = &self.highlighted();

        let iter = RG_WRAPPER.captures_iter(haystack).map(|m| {
            let (_, [word]) = m.extract();

            word.to_lowercase()
        });

        HashSet::from_iter(iter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_highlight_and_get_profine_words() {
        struct TestCase {
            line: &'static str,
            censored: &'static str,
            expected_bad_chars: Vec<usize>,
            expected_highlighted: &'static str,
            expected_profine_words: Vec<&'static str>,
            description: &'static str,
        }

        let test_cases = [
            TestCase {
                line: "good normal ok",
                censored: "good normal ok",
                expected_bad_chars: vec![],
                expected_highlighted: "good normal ok",
                expected_profine_words: vec![],
                description: "no profanity",
            },
            TestCase {
                line: "bad",
                censored: "***",
                expected_bad_chars: vec![0, 1, 2],
                expected_highlighted: "<tg-spoiler><u>bad</u></tg-spoiler>",
                expected_profine_words: vec!["bad"],
                description: "all censored",
            },
            TestCase {
                line: "badword",
                censored: "***word",
                expected_bad_chars: vec![0, 1, 2],
                expected_highlighted: "<tg-spoiler><u>bad</u></tg-spoiler>word",
                expected_profine_words: vec!["bad"],
                description: "partially censored",
            },
            TestCase {
                line: "",
                censored: "",
                expected_bad_chars: vec![],
                expected_highlighted: "",
                expected_profine_words: vec![],
                description: "empty string",
            },
            TestCase {
                line: "hëllo",
                censored: "***lo",
                expected_bad_chars: vec![0, 1, 2],
                expected_highlighted: "<tg-spoiler><u>hël</u></tg-spoiler>lo",
                expected_profine_words: vec!["hël"],
                description: "unicode extraction",
            },
            TestCase {
                line: "hello",
                censored: "h***o",
                expected_bad_chars: vec![1, 2, 3],
                expected_highlighted: "h<tg-spoiler><u>ell</u></tg-spoiler>o",
                expected_profine_words: vec!["ell"],
                description: "consecutive chars",
            },
            TestCase {
                line: "bad good worse",
                censored: "*** good *****",
                expected_bad_chars: vec![0, 1, 2, 9, 10, 11, 12, 13],
                expected_highlighted: "<tg-spoiler><u>bad</u></tg-spoiler> good <tg-spoiler><u>worse</u></tg-spoiler>",
                expected_profine_words: vec!["bad", "worse"],
                description: "multiple separate words",
            },
            TestCase {
                line: "héllo 世界",
                censored: "h**lo 世界",
                expected_bad_chars: vec![1, 2],
                expected_highlighted: "h<tg-spoiler><u>él</u></tg-spoiler>lo 世界",
                expected_profine_words: vec!["él"],
                description: "unicode highlighting",
            },
            TestCase {
                line: "good Bad normal worst ok bad",
                censored: "good *** normal ***** ok ***",
                expected_bad_chars: vec![5, 6, 7, 16, 17, 18, 19, 20, 25, 26, 27],
                expected_highlighted: "good <tg-spoiler><u>Bad</u></tg-spoiler> normal <tg-spoiler><u>worst</u></tg-spoiler> ok <tg-spoiler><u>bad</u></tg-spoiler>",
                expected_profine_words: vec!["bad", "worst"],
                description: "multiple bad words with case normalization",
            },
        ];

        for tc in test_cases {
            // Test extraction
            let bad_chars = CheckResult::extract_bad_chars(tc.line, tc.censored);
            assert_eq!(
                bad_chars, tc.expected_bad_chars,
                "bad_chars extraction failed for: {}",
                tc.description
            );

            // Test highlighting
            let line_result = LineResult {
                line: tc.line.to_owned(),
                bad_chars,
                ..Default::default()
            };
            let highlighted = line_result.highlighted();
            assert_eq!(
                highlighted, tc.expected_highlighted,
                "highlighting failed for: {}",
                tc.description
            );

            // Test get_profine_words
            let profine_words = line_result.get_profine_words();
            let expected_set: HashSet<String> = tc
                .expected_profine_words
                .iter()
                .map(|s| s.to_string())
                .collect();
            assert_eq!(
                profine_words, expected_set,
                "get_profine_words failed for: {}",
                tc.description
            );
        }
    }
}

#[derive(Clone, Copy, Deref, From, Default)]
pub struct TypeWrapper(Type);

impl TypeWrapper {
    fn name(&self) -> String {
        let typ = self.0;

        if typ.is(Type::SAFE) {
            return "🟢 safe".into();
        }

        if typ.isnt(*TYPE_THRESHOLD) {
            return "🟣 probably safe".into();
        }

        let emoji = if typ.is(Type::SEVERE) {
            '⛔'
        } else if typ.is(Type::MODERATE) {
            '🟠'
        } else if typ.is(Type::MILD) {
            '🟡'
        } else {
            '❔'
        };

        format!("{emoji} {typ:?}")
    }
}

impl Display for TypeWrapper {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[allow(dead_code)]
pub fn check_cases() {
    let cases = [];

    let _ = cases.map(|case| {
        let (censored, typ) = rustrict::Censor::from_str(case)
            .with_censor_first_character_threshold(Type::ANY)
            .with_censor_threshold(Type::ANY)
            .censor_and_analyze();

        println!("{case:?} {censored:?} {typ:?}");
    });
}

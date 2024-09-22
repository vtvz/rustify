use std::fmt::{Display, Formatter};
use std::slice::Iter;

use lazy_static::lazy_static;
use rustrict::{Trie, Type};
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

    pub fn check(lyrics: Vec<&str>) -> CheckResult {
        CheckResult::perform(lyrics)
    }
}

pub struct CheckResult {
    lines: Vec<LineResult>,
    pub typ: TypeWrapper,
}

impl IntoIterator for CheckResult {
    type IntoIter = <Vec<LineResult> as IntoIterator>::IntoIter;
    type Item = <Vec<LineResult> as IntoIterator>::Item;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        <Vec<LineResult> as IntoIterator>::into_iter(self.lines)
    }
}

impl CheckResult {
    pub fn iter(&self) -> Iter<LineResult> {
        self.lines.iter()
    }

    pub fn should_trigger(&self) -> bool {
        self.typ.is(*TYPE_TRIGGER)
    }

    fn perform(lyrics: Vec<&str>) -> Self {
        let checks: Vec<_> = lyrics
            .into_iter()
            .enumerate()
            .map(|(index, line)| {
                let line = html::escape(line);

                let (censored, typ) = rustrict::Censor::from_str(&line)
                    .with_censor_first_character_threshold(*TYPE_THRESHOLD)
                    .with_censor_threshold(*TYPE_THRESHOLD)
                    .censor_and_analyze();

                if typ.isnt(*TYPE_THRESHOLD) {
                    return LineResult {
                        no: index,
                        typ: Type::SAFE.into(),
                        line,
                        bad_chars: Default::default(),
                    };
                }

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

                LineResult {
                    no: index,
                    typ: typ.into(),
                    line,
                    bad_chars,
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
}

pub struct LineResult {
    pub no: usize,
    pub typ: TypeWrapper,
    pub line: String,
    pub bad_chars: Vec<usize>,
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
                    (false, true, false) => format!("||__{}__||", c),
                    (true, true, false) => format!("{}__||", c),
                    (false, true, true) => format!("||__{}", c),
                    _ => c.into(),
                }
            })
            .collect::<Vec<_>>()
            .join("")
    }
}

#[derive(Clone, Copy, Deref, From)]
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

        format!("{} {:?}", emoji, typ)
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

    cases.map(|case| {
        let (censored, typ) = rustrict::Censor::from_str(case)
            .with_censor_first_character_threshold(Type::ANY)
            .with_censor_threshold(Type::ANY)
            .censor_and_analyze();

        println!("{:?} {:?} {:?}", case, censored, typ);
    });
}

use lazy_static::lazy_static;
use std::fmt::{Display, Formatter};
use std::slice::Iter;

use rustrict::Type;
use teloxide::utils::markdown;

lazy_static! {
    static ref TYPE_THRESHOLD: Type = Type::ANY;
    static ref TYPE_CUSTOM: Type = Type::MODERATE & Type::EVASIVE;
    static ref TYPE_TRIGGER: Type = Type::INAPPROPRIATE | Type::EVASIVE;
}

pub struct Manager;

impl Manager {
    pub fn add_word(word: &str) {
        unsafe {
            rustrict::add_word(word, *TYPE_CUSTOM);
        }
        tracing::debug!(word, "Added custom word");
    }

    pub fn remove_word(word: &str) {
        unsafe {
            rustrict::add_word(word, Type::NONE);
        }
        tracing::debug!(word, "Removed custom word");
    }

    pub fn check(lyrics: Vec<String>) -> CheckResult {
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

    fn perform(lyrics: Vec<String>) -> Self {
        let checks: Vec<_> = lyrics
            .into_iter()
            .enumerate()
            .map(|(index, line)| {
                let line = markdown::escape(&line);

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
                    .into_iter()
                    .enumerate()
                    .filter_map(|(i, c)| {
                        if !censored.chars().nth(i).contains(&c) {
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
            .into_iter()
            .enumerate()
            .map(|(i, c)| {
                match (
                    i != 0 && self.bad_chars.contains(&(i - 1)),
                    self.bad_chars.contains(&i),
                    self.bad_chars.contains(&(i + 1)),
                ) {
                    (false, true, false) => format!("||{}||", c),
                    (true, true, false) => format!("{}||", c),
                    (false, true, true) => format!("||{}", c),
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

        if typ.isnt(*TYPE_THRESHOLD) {
            return "safe 🟢".into();
        }

        let (lvl, emoji) = if typ.is(Type::SEVERE) {
            ("severe", '⛔')
        } else if typ.is(Type::MODERATE) {
            ("moderate", '🟠')
        } else if typ.is(Type::MILD) {
            ("mild", '🟡')
        } else {
            ("undefined", '❔')
        };

        let mut types = vec![];

        if typ.is(Type::PROFANE) {
            types.push("profane");
        }

        if typ.is(Type::OFFENSIVE) {
            types.push("offensive");
        }

        if typ.is(Type::SEXUAL) {
            types.push("sexual");
        }

        if typ.is(Type::MEAN) {
            types.push("mean");
        }

        if typ.is(Type::EVASIVE) {
            types.push("evasive");
        }

        if typ.is(Type::SPAM) {
            types.push("spam");
        }

        format!("{} {} {}", lvl, types.join(" "), emoji)
    }
}

impl Display for TypeWrapper {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

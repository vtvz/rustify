use lazy_static::lazy_static;
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

    pub fn check(lyrics: Vec<String>) -> CheckResult {
        CheckResult::perform(lyrics)
    }
}

#[derive(IntoIterator)]
pub struct CheckResult(#[into_iterator] Vec<LineResult>, Type);

impl CheckResult {
    pub fn iter(&self) -> Iter<LineResult> {
        self.0.iter()
    }

    pub fn should_trigger(&self) -> bool {
        self.1.is(*TYPE_TRIGGER)
    }

    pub fn sum_type_name(&self) -> String {
        get_type_name(self.1)
    }

    fn perform(lyrics: Vec<String>) -> Self {
        let checks: Vec<_> = lyrics
            .into_iter()
            .enumerate()
            .map(|(index, line)| {
                let (censored, typ) = rustrict::Censor::from_str(&line)
                    .with_censor_first_character_threshold(*TYPE_THRESHOLD)
                    .with_censor_threshold(*TYPE_THRESHOLD)
                    .censor_and_analyze();

                if typ.isnt(*TYPE_THRESHOLD) {
                    return LineResult {
                        no: index,
                        typ: Type::SAFE,
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
                    typ,
                    line,
                    bad_chars,
                }
            })
            .collect();

        let sum_type = checks
            .iter()
            .map(|LineResult { typ, .. }| typ)
            .filter(|typ| typ.is(*TYPE_THRESHOLD))
            .fold(Type::NONE, |acc, typ| acc | *typ);

        Self(checks, sum_type)
    }
}

pub struct LineResult {
    pub no: usize,
    pub typ: Type,
    pub line: String,
    pub bad_chars: Vec<usize>,
}

impl LineResult {
    pub fn highlighted(&self) -> String {
        markdown::escape(self.line.as_str())
            .chars()
            .into_iter()
            .enumerate()
            .map(|(i, c)| {
                if self.bad_chars.contains(&i) {
                    format!("__{}__", c)
                } else {
                    c.into()
                }
            })
            .collect::<Vec<_>>()
            .join("")
            .replace("____", "")
    }

    pub fn get_type_name(&self) -> String {
        let typ = self.typ;

        get_type_name(typ)
    }
}

fn get_type_name(typ: Type) -> String {
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

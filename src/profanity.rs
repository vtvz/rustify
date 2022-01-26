use std::slice::Iter;

use rustrict::Type;
use teloxide::utils::markdown;

#[derive(IntoIterator)]
pub struct Check(Vec<LineResult>);

impl Check {
    pub fn iter(&self) -> Iter<LineResult> {
        self.0.iter()
    }

    pub fn sum_type(&self) -> Type {
        self.0
            .iter()
            .map(|LineResult { typ, .. }| typ)
            .filter(|typ| typ.isnt(Type::SAFE))
            .fold(Type::NONE, |acc, typ| acc | *typ)
    }

    pub fn sum_type_name(&self) -> String {
        get_type_name(self.sum_type())
    }

    pub fn perform(lyrics: Vec<String>) -> Self {
        let checks = lyrics
            .into_iter()
            .enumerate()
            .map(|(index, line)| {
                let (censored, typ) = rustrict::Censor::from_str(&line)
                    .with_censor_first_character_threshold(Type::ANY)
                    .with_censor_threshold(Type::ANY)
                    .censor_and_analyze();

                // safe || none || only spam
                if typ.is(Type::SAFE) || typ.is_empty() || (typ & !Type::SPAM).is_empty() {
                    return LineResult {
                        no: index,
                        typ: Type::SAFE,
                        line,
                        bad_chars: Default::default(),
                    };
                }

                let bad_chars = line
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
                    .collect::<Vec<_>>();

                LineResult {
                    no: index,
                    typ,
                    line,
                    bad_chars,
                }
            })
            .collect();

        Self(checks)
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
    if typ.is(Type::SAFE) || typ == Type::NONE {
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

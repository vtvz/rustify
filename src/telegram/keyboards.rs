use teloxide::types::{KeyboardButton, KeyboardMarkup, ReplyMarkup};

use crate::entity::prelude::UserLocale;

#[derive(Clone)]
pub enum StartKeyboard {
    Dislike,
    Stats,
    Details,
}

impl StartKeyboard {
    pub fn into_button(&self, locale: &str) -> KeyboardButton {
        let text = match self {
            StartKeyboard::Dislike => t!("start-keyboard.dislike", locale = locale),
            StartKeyboard::Stats => t!("start-keyboard.stats", locale = locale),
            StartKeyboard::Details => t!("start-keyboard.details", locale = locale),
        };

        KeyboardButton::new(text)
    }

    pub fn markup(locale: &str) -> ReplyMarkup {
        ReplyMarkup::Keyboard(
            KeyboardMarkup::new(vec![
                vec![Self::Dislike.into_button(locale)],
                vec![
                    Self::Stats.into_button(locale),
                    Self::Details.into_button(locale),
                ],
            ])
            .resize_keyboard(),
        )
    }

    pub fn from_str(text: &str, locale: &str) -> Option<StartKeyboard> {
        if text == t!("start-keyboard.dislike", locale = locale) {
            Some(StartKeyboard::Dislike)
        } else if text == t!("start-keyboard.stats", locale = locale) {
            Some(StartKeyboard::Stats)
        } else if text == t!("start-keyboard.details", locale = locale) {
            Some(StartKeyboard::Details)
        } else {
            None
        }
    }
}

#[derive(Clone)]
pub enum LanguageKeyboard {
    Russian,
    English,
}

impl LanguageKeyboard {
    pub fn into_button(&self) -> KeyboardButton {
        let text = match self {
            Self::English => t!("language.change", locale = "en"),
            Self::Russian => t!("language.change", locale = "ru"),
        };

        KeyboardButton::new(text)
    }

    pub fn into_locale(&self) -> UserLocale {
        match self {
            Self::English => UserLocale::English,
            Self::Russian => UserLocale::Russian,
        }
    }

    pub fn markup() -> ReplyMarkup {
        ReplyMarkup::Keyboard(
            KeyboardMarkup::new(vec![vec![
                Self::Russian.into_button(),
                Self::English.into_button(),
            ]])
            .resize_keyboard(),
        )
    }

    pub fn parse(text: &str) -> Option<Self> {
        if text == t!("language.change", locale = "en") {
            Some(Self::English)
        } else if text == t!("language.change", locale = "ru") {
            Some(Self::Russian)
        } else {
            None
        }
    }
}

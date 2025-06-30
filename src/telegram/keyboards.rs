use strum_macros::EnumString;
use teloxide::types::{KeyboardButton, KeyboardMarkup, ReplyMarkup};

#[derive(Clone, EnumString)]
pub enum StartKeyboard {
    #[strum(serialize = "👎 Dislike playing track")]
    Dislike,
    #[strum(serialize = "📈 Stats")]
    Stats,
    #[strum(serialize = "🎤 Details of track")]
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
}

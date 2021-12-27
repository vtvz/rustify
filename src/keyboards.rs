use strum_macros::{AsRefStr, EnumString};
use teloxide::types::{KeyboardButton, KeyboardMarkup, ReplyMarkup};

#[derive(Clone, EnumString, AsRefStr)]
pub enum StartKeyboard {
    #[strum(serialize = "👎 Dislike")]
    Dislike,
    #[strum(serialize = "📈 Stats")]
    Stats,
    #[strum(serialize = "🗑 Cleanup")]
    Cleanup,
}

impl From<StartKeyboard> for KeyboardButton {
    fn from(keyboard: StartKeyboard) -> Self {
        Self::new(keyboard.as_ref())
    }
}

impl StartKeyboard {
    pub fn markup() -> ReplyMarkup {
        ReplyMarkup::Keyboard(
            KeyboardMarkup::new(vec![
                vec![StartKeyboard::Dislike.into()],
                vec![StartKeyboard::Stats.into(), StartKeyboard::Cleanup.into()],
            ])
            .resize_keyboard(true),
        )
    }
}

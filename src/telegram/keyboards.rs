use strum_macros::{AsRefStr, EnumString};
use teloxide::types::{KeyboardButton, KeyboardMarkup, ReplyMarkup};

#[derive(Clone, EnumString, AsRefStr)]
pub enum StartKeyboard {
    #[strum(serialize = "👎 Dislike playing track")]
    Dislike,
    #[strum(serialize = "📈 Stats")]
    Stats,
    #[strum(serialize = "🎤 Details of track")]
    Details,
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
                vec![Self::Dislike.into()],
                vec![Self::Stats.into(), Self::Details.into()],
            ])
            .resize_keyboard(),
        )
    }
}

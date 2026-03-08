use std::borrow::Cow;

use teloxide::types::{InlineKeyboardButton, InlineKeyboardButtonKind};
use url::Url;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum InlineButtonsActions {
    ArtistPage(Url),
}

impl InlineButtonsActions {
    #[must_use]
    pub fn label(&self, locale: &str) -> Cow<'_, str> {
        match self {
            Self::ArtistPage(_) => t!("inline-buttons.artist-page", locale = locale),
        }
    }
}

impl InlineButtonsActions {
    #[must_use]
    pub fn into_inline_keyboard_button(self, locale: &str) -> InlineKeyboardButton {
        let label = self.label(locale);

        InlineKeyboardButton::new(label, self.clone().into())
    }
}

#[allow(clippy::from_over_into)]
impl Into<InlineKeyboardButtonKind> for InlineButtonsActions {
    fn into(self) -> InlineKeyboardButtonKind {
        match self {
            Self::ArtistPage(url) => InlineKeyboardButtonKind::Url(url),
        }
    }
}

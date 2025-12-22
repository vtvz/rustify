use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use teloxide::types::{InlineKeyboardButton, InlineKeyboardButtonKind};

use crate::entity::prelude::TrackStatus;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum InlineButtons {
    Dislike(String),
    Ignore(String),
    Analyze(String),
    Magic,
    SkippageEnable(bool),
    Recommendasion,
}

impl InlineButtons {
    #[must_use]
    pub fn label(&self, locale: &str) -> Cow<'_, str> {
        match self {
            InlineButtons::Dislike(_) => t!("inline-buttons.dislike", locale = locale),
            InlineButtons::Ignore(_) => t!("inline-buttons.ignore", locale = locale),
            InlineButtons::Analyze(_) => t!("inline-buttons.analyze", locale = locale),
            InlineButtons::Magic => t!("magic.button", locale = locale),
            InlineButtons::Recommendasion => t!("recommendasion.button", locale = locale),
            InlineButtons::SkippageEnable(to_enable) => {
                if *to_enable {
                    t!("skippage.enable-button", locale = locale)
                } else {
                    t!("skippage.disable-button", locale = locale)
                }
            },
        }
    }
}

impl InlineButtons {
    #[must_use]
    pub fn from_track_status(
        status: TrackStatus,
        track_id: &str,
        locale: &str,
    ) -> Vec<Vec<InlineKeyboardButton>> {
        match status {
            TrackStatus::None => {
                #[rustfmt::skip]
                vec![
                    vec![Self::Dislike(track_id.to_owned()).into_inline_keyboard_button(locale)],
                    vec![Self::Ignore(track_id.to_owned()).into_inline_keyboard_button(locale)],
                ]
            },
            TrackStatus::Disliked => {
                #[rustfmt::skip]
                vec![
                    vec![Self::Ignore(track_id.to_owned()).into_inline_keyboard_button(locale)],
                ]
            },
            TrackStatus::Ignore => {
                #[rustfmt::skip]
                vec![
                    vec![Self::Dislike(track_id.to_owned()).into_inline_keyboard_button(locale)],
                ]
            },
        }
    }
}

impl InlineButtons {
    #[must_use]
    pub fn into_inline_keyboard_button(self, locale: &str) -> InlineKeyboardButton {
        let label = self.label(locale);

        InlineKeyboardButton::new(label, self.clone().into())
    }
}

#[allow(clippy::from_over_into)]
impl Into<InlineKeyboardButtonKind> for InlineButtons {
    fn into(self) -> InlineKeyboardButtonKind {
        InlineKeyboardButtonKind::CallbackData(self.to_string())
    }
}

impl FromStr for InlineButtons {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

impl Display for InlineButtons {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            serde_json::to_string(self)
                .map_err(|_| std::fmt::Error)?
                .as_ref(),
        )
    }
}

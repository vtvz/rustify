use std::fmt::{Display, Formatter};
use std::str::FromStr;

use teloxide::types::{InlineKeyboardButton, InlineKeyboardButtonKind};

use crate::entity::prelude::TrackStatus;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum InlineButtons {
    Dislike(String),
    Ignore(String),
    Analyze(String),
}

impl InlineButtons {
    pub fn label(&self) -> &str {
        match self {
            InlineButtons::Dislike(_) => "Dislike 👎",
            InlineButtons::Ignore(_) => "Ignore text 🙈",
            InlineButtons::Analyze(_) => "Analyze text 🔍",
        }
    }
}

impl InlineButtons {
    pub fn from_track_status(
        status: TrackStatus,
        track_id: &str,
    ) -> Vec<Vec<InlineKeyboardButton>> {
        match status {
            TrackStatus::None => {
                #[rustfmt::skip]
                vec![
                    vec![Self::Dislike(track_id.to_owned()).into()],
                    vec![Self::Ignore(track_id.to_owned()).into()],
                ]
            },
            TrackStatus::Disliked => {
                #[rustfmt::skip]
                vec![
                    vec![Self::Ignore(track_id.to_owned()).into()],
                ]
            },
            TrackStatus::Ignore => {
                #[rustfmt::skip]
                vec![
                    vec![Self::Dislike(track_id.to_owned()).into()],
                ]
            },
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<InlineKeyboardButtonKind> for InlineButtons {
    fn into(self) -> InlineKeyboardButtonKind {
        InlineKeyboardButtonKind::CallbackData(self.to_string())
    }
}

#[allow(clippy::from_over_into)]
impl Into<InlineKeyboardButton> for InlineButtons {
    fn into(self) -> InlineKeyboardButton {
        let label = self.label();
        InlineKeyboardButton::new(label, self.clone().into())
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

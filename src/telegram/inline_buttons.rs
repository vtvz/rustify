use std::fmt::{Display, Formatter};
use std::str::FromStr;

use teloxide::types::{InlineKeyboardButton, InlineKeyboardButtonKind};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum InlineButtons {
    Cancel(String),
    Dislike(String),
    Ignore(String),
}

impl InlineButtons {
    pub fn label(&self) -> &str {
        match self {
            InlineButtons::Cancel(_) => "Cancel ↩",
            InlineButtons::Dislike(_) => "Dislike 👎",
            InlineButtons::Ignore(_) => "Ignore text 🙈",
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

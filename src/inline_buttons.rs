use std::fmt::{Display, Formatter};
use std::str::FromStr;
use teloxide::types::InlineKeyboardButtonKind;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum InlineButtons {
    Cancel(String),
}

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
                .or_else(|_| Err(std::fmt::Error))?
                .as_ref(),
        )
    }
}

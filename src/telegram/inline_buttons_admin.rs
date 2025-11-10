use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use teloxide::types::{InlineKeyboardButton, InlineKeyboardButtonKind};

use crate::entity::prelude::UserStatus;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum AdminInlineButtons {
    #[serde(rename = "wd")]
    RegenerateWordDefinition {
        #[serde(rename = "l")]
        locale: String,
        #[serde(rename = "w")]
        word: String,
    },
    #[serde(rename = "wp")]
    WordDefinitionsPage {
        #[serde(rename = "l")]
        locale: String,
        #[serde(rename = "p")]
        page: usize,
        #[serde(skip, default)]
        is_next: bool,
    },
    #[serde(rename = "aup")]
    AdminUsersPage {
        #[serde(rename = "p")]
        page: u64,

        #[serde(rename = "s")]
        sort_info: AdminUsersSortInfo,

        #[serde(rename = "f", skip_serializing_if = "Option::is_none")]
        status_filter: Option<UserStatus>,

        #[serde(skip, default)]
        button_type: AdminUsersPageButtonType,
    },
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct AdminUsersSortInfo {
    #[serde(rename = "s", default)]
    pub sort_by: AdminUsersSortBy,
    #[serde(rename = "o", default)]
    pub sort_order: AdminUsersSortOrder,

    // Only for display
    #[serde(skip, default)]
    pub sort_selected: bool,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub enum AdminUsersPageButtonType {
    #[serde(rename = "p")]
    Previous,
    #[serde(rename = "n")]
    Next,
    #[serde(rename = "s")]
    #[default]
    Sorting,
    #[serde(rename = "f")]
    Filter,
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AdminUsersSortBy {
    #[serde(rename = "c")]
    #[default]
    CreatedAt,
    #[serde(rename = "l")]
    LyricsChecked,
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AdminUsersSortOrder {
    #[serde(rename = "a")]
    Asc,
    #[serde(rename = "d")]
    #[default]
    Desc,
}

impl std::ops::Not for AdminUsersSortOrder {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Self::Asc => Self::Desc,
            Self::Desc => Self::Asc,
        }
    }
}

impl AdminInlineButtons {
    pub fn label(&self, _locale: &str) -> Cow<'_, str> {
        match self {
            Self::RegenerateWordDefinition { .. } => Cow::Borrowed("Regenerate 🔄"),
            Self::WordDefinitionsPage { page, is_next, .. } => {
                if *is_next {
                    Cow::Owned(format!("Page {} ▶", page + 1))
                } else {
                    Cow::Owned(format!("◀ Page {}", page + 1))
                }
            },
            Self::AdminUsersPage {
                page: _,
                button_type,
                sort_info,
                status_filter,
            } => match button_type {
                AdminUsersPageButtonType::Previous => Cow::Borrowed("◀ Previous"),
                AdminUsersPageButtonType::Next => Cow::Borrowed("Next ▶"),
                AdminUsersPageButtonType::Sorting => {
                    let arrow = if sort_info.sort_selected {
                        // We need to reverse order
                        // as this value represents "future" order
                        // if button be pressed
                        match !sort_info.sort_order {
                            AdminUsersSortOrder::Asc => " ▲",
                            AdminUsersSortOrder::Desc => " ▼",
                        }
                    } else {
                        ""
                    };

                    match sort_info.sort_by {
                        AdminUsersSortBy::CreatedAt => Cow::Owned(format!("Created{}", arrow)),
                        AdminUsersSortBy::LyricsChecked => Cow::Owned(format!("Lyrics{}", arrow)),
                    }
                },
                AdminUsersPageButtonType::Filter => {
                    if let Some(status) = status_filter {
                        Cow::Owned(format!("Next filter: {:?}", status))
                    } else {
                        Cow::Borrowed("Next filter: All")
                    }
                },
            },
        }
    }
}

impl AdminInlineButtons {
    pub fn into_inline_keyboard_button(self, locale: &str) -> InlineKeyboardButton {
        let label = self.label(locale);

        InlineKeyboardButton::new(label, self.clone().into())
    }
}

#[allow(clippy::from_over_into)]
impl Into<InlineKeyboardButtonKind> for AdminInlineButtons {
    fn into(self) -> InlineKeyboardButtonKind {
        InlineKeyboardButtonKind::CallbackData(self.to_string())
    }
}

impl FromStr for AdminInlineButtons {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

impl Display for AdminInlineButtons {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            serde_json::to_string(self)
                .map_err(|_| std::fmt::Error)?
                .as_ref(),
        )
    }
}

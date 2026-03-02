use sea_orm::Iterable as _;
use teloxide::payloads::{
    AnswerCallbackQuerySetters as _,
    EditMessageReplyMarkupSetters as _,
    SendMessageSetters as _,
};
use teloxide::prelude::Requester as _;
use teloxide::sugar::bot::BotMessagesExt as _;
use teloxide::types::{
    CallbackQuery,
    ChatId,
    InlineKeyboardButton,
    InlineKeyboardMarkup,
    ReplyMarkup,
};

use crate::app::App;
use crate::entity::prelude::UserAISlopDetection;
use crate::services::UserService;
use crate::telegram::handlers::HandleStatus;
use crate::telegram::inline_buttons::InlineButtons;
use crate::user::UserState;
use crate::utils::teloxide::CallbackQueryExt as _;

#[tracing::instrument(skip_all, fields(user_id = %state.user_id()))]
pub async fn handle_inline(
    app: &'static App,
    state: &UserState,
    q: CallbackQuery,
    status: UserAISlopDetection,
) -> anyhow::Result<()> {
    let Some(message) = q.get_message() else {
        app.bot()
            .answer_callback_query(q.id.clone())
            .text("Inaccessible Message")
            .await?;

        return Ok(());
    };

    app.bot().answer_callback_query(q.id).await?;

    if status != state.user().cfg_ai_slop_detection {
        UserService::set_cfg_ai_slop_detection(app.db(), state.user_id(), status).await?;

        app.bot()
            .edit_reply_markup(&message)
            .reply_markup(InlineKeyboardMarkup::new(get_keyboard(
                status,
                state.locale(),
            )))
            .await?;
    }

    Ok(())
}

#[must_use]
pub fn get_keyboard(
    current_setting: UserAISlopDetection,
    locale: &str,
) -> Vec<Vec<InlineKeyboardButton>> {
    UserAISlopDetection::iter()
        .map(|status| {
            vec![
                InlineButtons::AISlopDetection(status, current_setting == status)
                    .into_inline_keyboard_button(locale),
            ]
        })
        .collect()
}

#[tracing::instrument(skip_all, fields(user_id = %state.user_id()))]
pub async fn handle(
    app: &'static App,
    state: &UserState,
    chat_id: ChatId,
) -> anyhow::Result<HandleStatus> {
    app.bot()
        .send_message(
            chat_id,
            t!("ai-slop.setting-description", locale = state.locale()),
        )
        .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(
            get_keyboard(state.user().cfg_ai_slop_detection, state.locale()),
        )))
        .await?;

    Ok(HandleStatus::Handled)
}

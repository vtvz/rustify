use teloxide::types::{CallbackQuery, MaybeInaccessibleMessage, Message};

pub trait CallbackQueryExt {
    fn get_message(&self) -> Option<Message>;
}

impl CallbackQueryExt for CallbackQuery {
    fn get_message(&self) -> Option<Message> {
        let Some(MaybeInaccessibleMessage::Regular(message)) = self.message.clone() else {
            return None;
        };

        Some(*message)
    }
}

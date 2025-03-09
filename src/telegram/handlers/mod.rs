pub mod admin_commands;
pub mod commands;
pub mod inline_buttons;
pub mod keyboards;
pub mod message;
pub mod raw_message;
pub mod url;

pub enum HandleStatus {
    Handled,
    Skipped,
}

macro_rules! return_if_handled {
    ($handle:expr) => {
        if matches!($handle, HandleStatus::Handled) {
            return Ok(HandleStatus::Handled);
        }
    };
}

pub(crate) use return_if_handled;

pub mod commands;
pub mod inline_buttons;
pub mod keyboards;
pub mod raw_message;
pub mod url;

pub enum HandleStatus {
    Handled,
    Skipped,
}

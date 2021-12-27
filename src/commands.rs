use teloxide::utils::command::BotCommand;

#[derive(BotCommand, PartialEq, Debug)]
#[command(rename = "lowercase")]
pub enum Command {
    #[command(description = "start")]
    Start,
    #[command(description = "echo back the message")]
    Echo(String),
    #[command(description = "show this help")]
    Help,
}

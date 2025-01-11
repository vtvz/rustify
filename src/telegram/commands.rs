use teloxide::utils::command::BotCommands;

#[derive(BotCommands, PartialEq, Eq, Debug)]
#[command(rename_rule = "snake_case", parse_with = "split")]
pub enum Command {
    #[command(description = "start")]
    Start,
    #[command(description = "show keyboard")]
    Keyboard,
    #[command(description = "dislike current track")]
    Dislike,
    #[command(description = "like current track")]
    Like,
    #[command(description = "delete disliked tracks from your playlists")]
    Cleanup,
    #[command(description = "show details about currently playing track")]
    Details,
    #[command(description = "show statistics about disliked tracks")]
    Stats,
    #[command(description = "login to spotify")]
    Register,
    #[command(description = "toggle setting of skipping disliked tracks")]
    ToggleTrackSkip,
    #[command(description = "toggle setting of profanity check")]
    ToggleProfanityCheck,
    #[command(description = "show this help")]
    Help,

    #[command(
        description = "add word to whitelist",
        rename = "add_word_to_whitelist"
    )]
    AddWhitelistWord { word: String },

    #[command(
        description = "remove word from whitelist",
        rename = "remove_word_from_whitelist"
    )]
    RemoveWhitelistWord { word: String },

    #[command(
        description = "list words in whitelist",
        rename = "list_words_in_whitelist"
    )]
    ListWhitelistWords,

    #[command(hide)]
    Whitelist(String, String),
}

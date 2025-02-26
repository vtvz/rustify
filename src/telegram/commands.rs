use std::fmt::Formatter;

use teloxide::utils::command::BotCommands;

lazy_static::lazy_static! {
    pub static ref ADD_WHITELIST_WORD_COMMAND: String = Command::AddWhitelistWord {
        word: String::new()
    }
    .to_string();

    pub static ref REMOVE_WHITELIST_WORD_COMMAND: String = Command::RemoveWhitelistWord {
        word: String::new()
    }
    .to_string();

    pub static ref LIST_WHITELIST_WORDS_COMMAND: String = Command::ListWhitelistWords.to_string();

    pub static ref SET_ANALYSIS_LANGUAGE_COMMAND: String = Command::SetAnalysisLanguage { language: String::new() }.to_string();
}

#[derive(BotCommands, PartialEq, Eq, Debug)]
#[command(rename_rule = "snake_case", parse_with = "split")]
pub enum Command {
    #[command(description = "show this help")]
    Help,

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
    #[command(description = "set language for analysis results")]
    SetAnalysisLanguage { language: String },

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
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            Command::Start => "start",
            Command::Keyboard => "keyboard",
            Command::Dislike => "dislike",
            Command::Like => "like",
            Command::Cleanup => "cleanup",
            Command::Details => "details",
            Command::Stats => "stats",
            Command::Register => "register",
            Command::ToggleTrackSkip => "toggle_track_skip",
            Command::ToggleProfanityCheck => "toggle_profanity_check",
            Command::SetAnalysisLanguage { .. } => "set_analysis_language",
            Command::Help => "help",
            Command::AddWhitelistWord { .. } => "add_word_to_whitelist",
            Command::RemoveWhitelistWord { .. } => "remove_word_from_whitelist",
            Command::ListWhitelistWords => "list_words_in_whitelist",
        };

        f.write_str(string)
    }
}

#[derive(BotCommands, PartialEq, Eq, Debug)]
#[command(rename_rule = "snake_case", parse_with = "split")]
pub enum AdminCommand {
    #[command(description = "show this help")]
    Admin,

    #[command(description = "manage user whitelist")]
    Whitelist(String, String),

    #[command(description = "show global statistics")]
    AdminStats,
}

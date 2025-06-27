use std::fmt::Formatter;

use teloxide::utils::command::BotCommands;

lazy_static::lazy_static! {
    pub static ref ADD_WHITELIST_WORD_COMMAND: String = UserCommand::AddWhitelistWord {
        word: String::new()
    }
    .to_string();

    pub static ref REMOVE_WHITELIST_WORD_COMMAND: String = UserCommand::RemoveWhitelistWord {
        word: String::new()
    }
    .to_string();

    pub static ref LIST_WHITELIST_WORDS_COMMAND: String = UserCommand::ListWhitelistWords.to_string();

    pub static ref SET_ANALYSIS_LANGUAGE_COMMAND: String = UserCommand::SetAnalysisLanguage { language: String::new() }.to_string();
}

#[derive(BotCommands, PartialEq, Eq, Debug)]
#[command(rename_rule = "snake_case", parse_with = "split")]
pub enum UserCommand {
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

    #[command(description = "create or refresh Magic playlist")]
    Magic,

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

    #[command(
        description = "allows you to skip tracks you've already listened. Pass days to remember"
    )]
    Skippage { days: String },
}

impl std::fmt::Display for UserCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            UserCommand::Start => "start",
            UserCommand::Keyboard => "keyboard",
            UserCommand::Dislike => "dislike",
            UserCommand::Like => "like",
            UserCommand::Cleanup => "cleanup",
            UserCommand::Details => "details",
            UserCommand::Stats => "stats",
            UserCommand::Register => "register",
            UserCommand::ToggleTrackSkip => "toggle_track_skip",
            UserCommand::ToggleProfanityCheck => "toggle_profanity_check",
            UserCommand::SetAnalysisLanguage { .. } => "set_analysis_language",
            UserCommand::Help => "help",
            UserCommand::AddWhitelistWord { .. } => "add_word_to_whitelist",
            UserCommand::RemoveWhitelistWord { .. } => "remove_word_from_whitelist",
            UserCommand::ListWhitelistWords => "list_words_in_whitelist",
            UserCommand::Magic => "magic",
            UserCommand::Skippage { .. } => "skippage",
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
    GlobalStats,

    #[command(description = "get analyze prompt")]
    GetAnalyzePrompt,
}

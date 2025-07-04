use std::fmt::Formatter;

use teloxide::utils::command::BotCommands;

#[derive(BotCommands, PartialEq, Eq, Debug)]
#[command(rename_rule = "snake_case", parse_with = "split")]
pub enum UserCommand {
    #[command(description = "Show this help")]
    Help,

    #[command(description = "Start")]
    Start,
    #[command(description = "Show keyboard")]
    Keyboard,
    #[command(description = "Dislike current track")]
    Dislike,
    #[command(description = "Like current track")]
    Like,
    #[command(description = "Delete disliked tracks from your playlists")]
    Cleanup,
    #[command(description = "Show details about currently playing track")]
    Details,
    #[command(description = "Show statistics about disliked tracks")]
    Stats,
    #[command(description = "Login to spotify")]
    Register,

    #[command(description = "Toggle setting of skipping disliked tracks")]
    ToggleTrackSkip,
    #[command(description = "Toggle setting of profanity check")]
    ToggleProfanityCheck,
    #[command(description = "Set language for analysis results")]
    SetAnalysisLanguage { language: String },

    #[command(description = "Create or refresh Magic playlist")]
    Magic,

    #[command(
        description = "Add word to whitelist",
        rename = "add_word_to_whitelist"
    )]
    AddWhitelistWord { word: String },

    #[command(
        description = "Remove word from whitelist",
        rename = "remove_word_from_whitelist"
    )]
    RemoveWhitelistWord { word: String },

    #[command(
        description = "List words in whitelist",
        rename = "list_words_in_whitelist"
    )]
    ListWhitelistWords,

    #[command(
        description = "Allows you to skip tracks you've already listened. Pass days to remember"
    )]
    Skippage { days: String },
}

pub enum UserCommandDisplay {
    Start,
    Keyboard,
    Dislike,
    Like,
    Cleanup,
    Details,
    Stats,
    Register,
    ToggleTrackSkip,
    ToggleProfanityCheck,
    SetAnalysisLanguage,
    Help,
    AddWhitelistWord,
    RemoveWhitelistWord,
    ListWhitelistWords,
    Magic,
    Skippage,
}

impl std::fmt::Display for UserCommandDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            Self::Start => "start",
            Self::Keyboard => "keyboard",
            Self::Dislike => "dislike",
            Self::Like => "like",
            Self::Cleanup => "cleanup",
            Self::Details => "details",
            Self::Stats => "stats",
            Self::Register => "register",
            Self::ToggleTrackSkip => "toggle_track_skip",
            Self::ToggleProfanityCheck => "toggle_profanity_check",
            Self::SetAnalysisLanguage => "set_analysis_language",
            Self::Help => "help",
            Self::AddWhitelistWord => "add_word_to_whitelist",
            Self::RemoveWhitelistWord => "remove_word_from_whitelist",
            Self::ListWhitelistWords => "list_words_in_whitelist",
            Self::Magic => "magic",
            Self::Skippage => "skippage",
        };

        f.write_str(string)
    }
}

#[derive(BotCommands, PartialEq, Eq, Debug)]
#[command(rename_rule = "snake_case", parse_with = "split")]
pub enum AdminCommand {
    #[command(description = "Show this help")]
    Admin,

    #[command(description = "Manage user whitelist")]
    Whitelist(String, String),

    #[command(description = "Show global statistics")]
    GlobalStats,

    #[command(description = "Get analyze prompt")]
    GetAnalyzePrompt,
}

pub enum AdminCommandDisplay {
    Admin,
    Whitelist,
    GlobalStats,
    GetAnalyzePrompt,
}

impl std::fmt::Display for AdminCommandDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            AdminCommandDisplay::Admin => "admin",
            AdminCommandDisplay::Whitelist => "whitelist",
            AdminCommandDisplay::GlobalStats => "global_stats",
            AdminCommandDisplay::GetAnalyzePrompt => "get_analyze_prompt",
        };

        f.write_str(string)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_user_commands() {
        let user_command = UserCommand::Start;

        match user_command {
            UserCommand::Start => UserCommandDisplay::Start,
            UserCommand::Keyboard => UserCommandDisplay::Keyboard,
            UserCommand::Dislike => UserCommandDisplay::Dislike,
            UserCommand::Like => UserCommandDisplay::Like,
            UserCommand::Cleanup => UserCommandDisplay::Cleanup,
            UserCommand::Details => UserCommandDisplay::Details,
            UserCommand::Stats => UserCommandDisplay::Stats,
            UserCommand::Register => UserCommandDisplay::Register,
            UserCommand::ToggleTrackSkip => UserCommandDisplay::ToggleTrackSkip,
            UserCommand::ToggleProfanityCheck => UserCommandDisplay::ToggleProfanityCheck,
            UserCommand::SetAnalysisLanguage { .. } => UserCommandDisplay::SetAnalysisLanguage,
            UserCommand::Help => UserCommandDisplay::Help,
            UserCommand::AddWhitelistWord { .. } => UserCommandDisplay::AddWhitelistWord,
            UserCommand::RemoveWhitelistWord { .. } => UserCommandDisplay::RemoveWhitelistWord,
            UserCommand::ListWhitelistWords => UserCommandDisplay::ListWhitelistWords,
            UserCommand::Magic => UserCommandDisplay::Magic,
            UserCommand::Skippage { .. } => UserCommandDisplay::Skippage,
        };
    }

    #[test]
    fn check_admin_commands() {
        let admin_command = AdminCommand::Admin;

        match admin_command {
            AdminCommand::Admin => AdminCommandDisplay::Admin,
            AdminCommand::Whitelist(..) => AdminCommandDisplay::Whitelist,
            AdminCommand::GlobalStats => AdminCommandDisplay::GlobalStats,
            AdminCommand::GetAnalyzePrompt => AdminCommandDisplay::GetAnalyzePrompt,
        };
    }
}

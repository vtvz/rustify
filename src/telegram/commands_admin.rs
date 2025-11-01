use std::fmt::Formatter;

use teloxide::utils::command::BotCommands;

#[derive(BotCommands, PartialEq, Eq, Debug)]
#[command(rename_rule = "snake_case", parse_with = "split")]
pub enum AdminCommand {
    #[command(description = "Show this help")]
    Admin,

    #[command(description = "Show global statistics")]
    GlobalStats,

    #[command(description = "Broadcast a message to all users")]
    Broadcast { locale: String },

    #[command(description = "Get word definition")]
    GetWordDefinition { locale: String, word: String },

    #[command(description = "Reset word definition and generate a new one")]
    ResetWordDefinition { locale: String, word: String },

    #[command(description = "List word definitions by locale (en, ru, etc)")]
    ListWordDefinitions { locale: String },

    #[command(description = "List users")]
    Users { user_id: String },
}

pub enum AdminCommandDisplay {
    Admin,
    GlobalStats,
    Broadcast,
    GetWordDefinition,
    ResetWordDefinition,
    ListWordDefinitions,
    Users,
}

impl std::fmt::Display for AdminCommandDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            AdminCommandDisplay::Admin => "admin",
            AdminCommandDisplay::GlobalStats => "global_stats",
            AdminCommandDisplay::Broadcast => "broadcast",
            AdminCommandDisplay::GetWordDefinition => "get_word_definition",
            AdminCommandDisplay::ResetWordDefinition => "reset_word_definition",
            AdminCommandDisplay::ListWordDefinitions => "list_word_definitions",
            AdminCommandDisplay::Users => "users",
        };

        f.write_str(string)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn check_admin_commands() {
        let admin_command = AdminCommand::Admin;

        match admin_command {
            AdminCommand::Admin => AdminCommandDisplay::Admin,
            AdminCommand::GlobalStats => AdminCommandDisplay::GlobalStats,
            AdminCommand::Broadcast { .. } => AdminCommandDisplay::Broadcast,
            AdminCommand::GetWordDefinition { .. } => AdminCommandDisplay::GetWordDefinition,
            AdminCommand::ResetWordDefinition { .. } => AdminCommandDisplay::ResetWordDefinition,
            AdminCommand::ListWordDefinitions { .. } => AdminCommandDisplay::ListWordDefinitions,
            AdminCommand::Users { .. } => AdminCommandDisplay::Users,
        };
    }
}

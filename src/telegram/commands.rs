use std::collections::HashMap;
use std::fmt::Formatter;
use std::sync::RwLock;

use strum_macros::EnumIter;
use teloxide::types::BotCommand;
use teloxide::utils::command::{BotCommands, CommandDescription, CommandDescriptions};

#[derive(BotCommands, PartialEq, Eq, Debug, EnumIter)]
#[command(rename_rule = "snake_case")]
pub enum UserCommand {
    #[command(description = "command.help")]
    Help,

    #[command(description = "command.start")]
    Start,

    #[command(description = "command.language")]
    Language,

    #[command(description = "command.keyboard")]
    Keyboard,
    #[command(description = "command.dislike")]
    Dislike,
    #[command(description = "command.like")]
    Like,
    #[command(description = "command.recommendasion")]
    Recommendasion,
    #[command(description = "command.details")]
    Details,
    #[command(description = "command.stats")]
    Stats,
    #[command(description = "command.register")]
    Register,

    #[command(description = "command.toggle-track-skip")]
    ToggleTrackSkip,
    #[command(description = "command.toggle-profanity-check")]
    ToggleProfanityCheck,

    #[command(description = "command.magic")]
    Magic,

    #[command(
        description = "command.add-whitelist-word",
        rename = "add_word_to_whitelist"
    )]
    AddWhitelistWord { word: String },

    #[command(
        description = "command.remove-whitelist-word",
        rename = "remove_word_from_whitelist"
    )]
    RemoveWhitelistWord { word: String },

    #[command(
        description = "command.list-whitelist-words",
        rename = "list_words_in_whitelist"
    )]
    ListWhitelistWords,

    #[command(description = "command.skippage")]
    Skippage { days: String },
}

impl UserCommand {
    pub fn localized_bot_commands(locale: &str) -> Vec<BotCommand> {
        let commands = Self::bot_commands();

        commands
            .into_iter()
            .map(|command| {
                let description = t!(command.description.clone(), locale = locale);
                command.description(description.to_string())
            })
            .collect()
    }

    pub fn localized_descriptions(locale: &str) -> CommandDescriptions<'static> {
        lazy_static::lazy_static! {
            static ref CACHE: RwLock<HashMap<String, &'static [CommandDescription<'static>]>> = RwLock::new(HashMap::new());
        }

        let entry = { CACHE.read().expect("Lock is poisoned").get(locale).copied() };

        match entry {
            Some(descriptions) => CommandDescriptions::new(descriptions),

            None => {
                let descriptions: Vec<_> = Self::bot_commands()
                    .into_iter()
                    .map(|command| {
                        let description = t!(&command.description, locale = locale);
                        let command_str = Box::leak(command.command.into_boxed_str());
                        let description_str = Box::leak(description.to_string().into_boxed_str());
                        CommandDescription {
                            prefix: "",
                            command: command_str,
                            aliases: &[],
                            description: description_str,
                        }
                    })
                    .collect();

                let descriptions_static = Box::leak(descriptions.into_boxed_slice());

                CACHE
                    .write()
                    .expect("Lock is poisoned")
                    .insert(locale.into(), descriptions_static);

                CommandDescriptions::new(descriptions_static)
            },
        }
    }
}

pub enum UserCommandDisplay {
    Start,
    Keyboard,
    Dislike,
    Like,
    Details,
    Stats,
    Register,
    ToggleTrackSkip,
    ToggleProfanityCheck,
    Help,
    AddWhitelistWord,
    RemoveWhitelistWord,
    ListWhitelistWords,
    Magic,
    Skippage,
    Language,
    Recommendasion,
}

impl std::fmt::Display for UserCommandDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            Self::Start => "start",
            Self::Keyboard => "keyboard",
            Self::Dislike => "dislike",
            Self::Like => "like",
            Self::Details => "details",
            Self::Stats => "stats",
            Self::Register => "register",
            Self::ToggleTrackSkip => "toggle_track_skip",
            Self::ToggleProfanityCheck => "toggle_profanity_check",
            Self::Help => "help",
            Self::AddWhitelistWord => "add_word_to_whitelist",
            Self::RemoveWhitelistWord => "remove_word_from_whitelist",
            Self::ListWhitelistWords => "list_words_in_whitelist",
            Self::Magic => "magic",
            Self::Skippage => "skippage",
            Self::Language => "language",
            Self::Recommendasion => "recommendasion",
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
            UserCommand::Recommendasion => UserCommandDisplay::Recommendasion,
            UserCommand::Details => UserCommandDisplay::Details,
            UserCommand::Stats => UserCommandDisplay::Stats,
            UserCommand::Register => UserCommandDisplay::Register,
            UserCommand::ToggleTrackSkip => UserCommandDisplay::ToggleTrackSkip,
            UserCommand::ToggleProfanityCheck => UserCommandDisplay::ToggleProfanityCheck,
            UserCommand::Help => UserCommandDisplay::Help,
            UserCommand::AddWhitelistWord { .. } => UserCommandDisplay::AddWhitelistWord,
            UserCommand::RemoveWhitelistWord { .. } => UserCommandDisplay::RemoveWhitelistWord,
            UserCommand::ListWhitelistWords => UserCommandDisplay::ListWhitelistWords,
            UserCommand::Magic => UserCommandDisplay::Magic,
            UserCommand::Skippage { .. } => UserCommandDisplay::Skippage,
            UserCommand::Language => UserCommandDisplay::Language,
        };
    }

    // Tests for localized_bot_commands()
    #[test]
    fn test_localized_bot_commands() {
        let locales = vec!["en", "ru"];

        for locale in locales {
            let commands = UserCommand::localized_bot_commands(locale);
            // Should return a non-empty list
            assert!(!commands.is_empty());
            // Each command should have a non-empty description
            for command in commands {
                assert!(!command.description.is_empty());
            }
        }
    }

    // Tests for localized_descriptions()
    #[test]
    fn test_localized_descriptions() {
        let locales = vec!["en", "ru"];
        for locale in locales {
            let _descriptions = UserCommand::localized_descriptions(locale);
        }
    }

    #[test]
    fn test_localized_descriptions_caching() {
        let locales = vec!["en", "ru"];
        for locale in locales {
            let _descriptions1 = UserCommand::localized_descriptions(locale);
            let _descriptions2 = UserCommand::localized_descriptions(locale);
        }
    }
}

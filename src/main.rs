#![warn(clippy::unwrap_used)]
#![feature(
    stmt_expr_attributes,
    box_patterns,
    closure_track_caller,
    error_generic_member_access
)]

use clap::Parser;

#[macro_use]
extern crate derive_more;
#[macro_use]
extern crate serde;
#[macro_use]
extern crate rust_i18n;

pub mod app;
pub mod cache;
pub mod cli;
pub mod entity;
pub mod error_handler;
pub mod logger;
pub mod lyrics;
pub mod magic_service;
pub mod metrics;
pub mod profanity;
pub mod queue;
pub mod recommendasion_service;
pub mod rickroll;
pub mod serde_utils;
pub mod skippage_service;
pub mod spotify;
pub mod spotify_auth_service;
pub mod telegram;
pub mod tick;
pub mod track_status_service;
pub mod user;
pub mod user_service;
pub mod user_word_whitelist_service;
pub mod utils;
pub mod whitelist;
pub mod word_definition_service;
pub mod word_stats_service;
pub mod workers;

rust_i18n::i18n!("locales", fallback = "en");

#[derive(Parser)]
#[command(name = "rustify")]
enum CliCommands {
    /// Run the Telegram bot worker
    Bot,
    /// Run the track checking worker
    TrackCheck,
    /// Run the background queues worker
    Queues,
    /// Manage users
    #[command(subcommand)]
    Users(cli::users::UsersCommands),
}

#[tokio::main(worker_threads = 4)]
async fn main() {
    let command = CliCommands::parse();

    match command {
        CliCommands::Bot => workers::bot::work().await,
        CliCommands::TrackCheck => workers::track_check::work().await,
        CliCommands::Queues => workers::queues::work().await,
        CliCommands::Users(cmd) => cli::users::run(cmd).await,
    }
}

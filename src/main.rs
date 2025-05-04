#![warn(clippy::unwrap_used)]
#![feature(
    stmt_expr_attributes,
    box_patterns,
    closure_track_caller,
    error_generic_member_access
)]

use std::env;

#[macro_use]
extern crate derive_more;
#[macro_use]
extern crate serde;

pub mod app;
pub mod cache;
pub mod entity;
pub mod logger;
pub mod lyrics;
pub mod metrics;
pub mod profanity;
pub mod queue;
pub mod rickroll;
pub mod serde_utils;
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
pub mod workers;

#[tokio::main(worker_threads = 4)]
async fn main() {
    let mut args = env::args();
    args.next(); // skip

    let arg: String = args.next().unwrap_or_default();

    if &arg == "queues" {
        workers::queues::work().await;
    } else if &arg == "track_check" {
        workers::track_check::work().await;
    } else if &arg == "bot" {
        workers::bot::work().await;
    } else {
        panic!("Provide command");
    }
}

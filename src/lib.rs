#![allow(clippy::explicit_auto_deref)]
#![warn(clippy::unwrap_used)]
#![feature(
    stmt_expr_attributes,
    box_patterns,
    closure_track_caller,
    error_generic_member_access
)]

#[macro_use]
extern crate derive_more;
#[macro_use]
extern crate serde;

pub mod entity;
pub mod logger;
pub mod lyrics;
pub mod metrics;
pub mod profanity;
pub mod serde_utils;
pub mod spotify;
pub mod spotify_auth_service;
pub mod state;
pub mod telegram;
pub mod tick;
pub mod track_status_service;
pub mod user_service;
pub mod utils;
pub mod whitelist;

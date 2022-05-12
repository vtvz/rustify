use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use indoc::formatdoc;
use reqwest::{Response, StatusCode};
use rspotify::clients::OAuthClient;
use rspotify::http::HttpError;
use rspotify::model::{
    Context as SpotifyContext,
    FullTrack,
    PlayableId,
    PlaylistId,
    TrackId,
    Type as SpotifyType,
};
use rspotify::ClientError;
use rustrict::Type;
use sea_orm::DbConn;
use strum_macros::Display;
use teloxide::prelude::*;
use teloxide::types::{ChatId, InlineKeyboardMarkup, ParseMode, ReplyMarkup};
use teloxide::ApiError;
use tokio::sync::{broadcast, Semaphore};
use tokio::time::Instant;

use crate::entity::prelude::*;
use crate::errors::{Context, GenericError, GenericResult};
use crate::spotify::CurrentlyPlaying;
use crate::spotify_auth_service::SpotifyAuthService;
use crate::telegram::inline_buttons::InlineButtons;
use crate::track_status_service::TrackStatusService;
use crate::user_service::UserService;
use crate::{lyrics, profanity, spotify, state, telegram, utils};

const CHECK_INTERVAL: u64 = 3;
const PARALLEL_CHECKS: usize = 2;

#[tracing::instrument(skip_all, fields(user_id = %state.user_id))]
async fn handle_telegram_error(
    state: &state::UserState,
    result: Result<Message, teloxide::RequestError>,
) -> GenericResult<()> {
    if let Err(teloxide::RequestError::Api(ApiError::BotBlocked | ApiError::NotFound)) = result {
        UserService::set_status(&state.app.db, &state.user_id, UserStatus::Blocked).await?;
    }

    result?;

    Ok(())
}

#[derive(Default)]
struct CheckBadWordsResult {
    skipped: bool,
    found: bool,
    profane: bool,
    provider: Option<lyrics::Provider>,
}

#[tracing::instrument(
    skip_all,
    fields(
        track_id = %spotify::get_track_id(track),
        track_name = %spotify::create_track_name(track),
    )
)]
async fn check_bad_words(
    state: &state::UserState,
    track: &FullTrack,
) -> GenericResult<CheckBadWordsResult> {
    let mut ret = CheckBadWordsResult::default();

    let Some(hit) = state.app.lyrics.search_for_track(track).await? else {
        return Ok(ret);
    };

    ret.provider = Some(hit.provider());
    ret.found = true;

    if hit.language() != "en" {
        tracing::trace!(language = hit.language(), "Track has non English lyrics",);

        ret.skipped = true;
        return Ok(ret);
    }

    let check = profanity::Manager::check(hit.lyrics());

    if !check.should_trigger() {
        return Ok(ret);
    }

    let bad_lines: Vec<_> = check
        .into_iter()
        .filter(|profanity::LineResult { typ, .. }| !typ.is(Type::SAFE))
        .map(|line: profanity::LineResult| {
            format!(
                "`{}:` {}, `[{}]`",
                hit.line_index_name(line.no),
                line.highlighted(),
                line.typ
            )
        })
        .collect();

    if bad_lines.is_empty() {
        return Ok(ret);
    }

    ret.profane = true;

    let mut lines = bad_lines.len();
    let message = loop {
        let message = formatdoc!(
            "
                Current song \\({track_name}\\) probably has bad words \\(ignore in case of false positive\\):
                
                {bad_lines}
                
                {genius}
            ",
            track_name = spotify::create_track_tg_link(track),
            bad_lines = bad_lines[0..lines].join("\n"),
            genius = hit.tg_link(true)
        );

        if message.len() <= telegram::MESSAGE_MAX_LEN {
            break message;
        }

        lines -= 1;
    };

    let result: Result<Message, teloxide::RequestError> = state
        .app
        .bot
        .send_message(ChatId(state.user_id.parse()?), message)
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(
            #[rustfmt::skip]
            vec![
                vec![InlineButtons::Dislike(spotify::get_track_id(track)).into()],
                vec![InlineButtons::Ignore(spotify::get_track_id(track)).into()],
            ],
        )))
        .send()
        .await;

    handle_telegram_error(state, result).await.map(|_| ret)
}

#[tracing::instrument(
    skip_all,
    fields(
        track_id = %spotify::get_track_id(track),
        track_name = %spotify::create_track_name(track),
    )
)]
async fn handle_disliked_track(
    state: &state::UserState,
    track: &FullTrack,
    context: Option<&SpotifyContext>,
) -> GenericResult<()> {
    if state.is_spotify_premium() {
        let spotify = state.spotify.read().await;

        spotify
            .next_track(None)
            .await
            .context("Skip current track")?;

        let track_id = spotify::get_track_id(track);
        TrackStatusService::increase_skips(&state.app.db, &state.user_id, &track_id).await?;

        let Some(context) = context else {
            return Ok(());
        };

        match context._type {
            SpotifyType::Playlist => {
                let track_id = TrackId::from_str(&track_id)?;
                let hate: Option<&dyn PlayableId> = Some(&track_id);

                let res = spotify
                    .playlist_remove_all_occurrences_of_items(
                        &PlaylistId::from_str(&context.uri)?,
                        hate,
                        None,
                    )
                    .await;

                // It's a bit too much to check if user owns this playlist
                if res.is_ok() {
                    UserService::increase_stats_query(&state.user_id)
                        .removed_playlists(1)
                        .exec(&state.app.db)
                        .await?;
                }
            },

            SpotifyType::Collection => {
                let track_id = TrackId::from_str(&track_id)?;

                spotify
                    .current_user_saved_tracks_delete(Some(&track_id))
                    .await?;

                UserService::increase_stats_query(&state.user_id)
                    .removed_collection(1)
                    .exec(&state.app.db)
                    .await?;
            },
            _ => {},
        }

        return Ok(());
    }

    let message = format!(
        "Current song \\({track_name}\\) was disliked, but I cannot skip it...",
        track_name = spotify::create_track_tg_link(track),
    );

    let result = state
        .app
        .bot
        .send_message(ChatId(state.user_id.parse()?), message)
        .parse_mode(ParseMode::MarkdownV2)
        .send()
        .await;

    handle_telegram_error(state, result).await
}

#[tracing::instrument(skip_all, fields(user_id = %user_id))]
async fn handle_too_many_requests(
    db: &DbConn,
    user_id: &str,
    response: &Response,
) -> GenericResult<()> {
    if response.status() != StatusCode::TOO_MANY_REQUESTS {
        return Ok(());
    }

    tracing::info!("User got a 429 error (too many requests)");

    let header = response
        .headers()
        .get("Retry-After")
        .context("Need Retry-After header to proceed")?;

    let retry_after: i64 = header.to_str()?.parse()?;

    SpotifyAuthService::suspend_for(db, user_id, chrono::Duration::seconds(retry_after)).await?;

    Ok(())
}

#[derive(Clone, Display)]
enum CheckPlayingForUserResult {
    #[strum(serialize = "Skip same track")]
    SkipSame,
    #[strum(serialize = "Complete check")]
    Complete,
    #[strum(serialize = "Current track is on pause {1}")]
    None(spotify::CurrentlyPlayingNoneReason),
}

#[tracing::instrument(skip_all, fields(user_id = %user_id))]
async fn check_playing_for_user(
    app_state: &'static state::AppState,
    user_id: &str,
) -> GenericResult<CheckPlayingForUserResult> {
    let res = app_state
        .user_state(user_id)
        .await
        .context("Get user state");

    let state = match res {
        Err(GenericError::RspotifyClientError(ClientError::Http(box HttpError::StatusCode(
            ref response,
        )))) => {
            if let Err(err) = handle_too_many_requests(&app_state.db, user_id, response).await {
                tracing::error!(err = ?err.anyhow(), "Something went wrong");
            }

            res?
        },
        Err(err) => return Err(err),
        Ok(state) => state,
    };

    let playing = spotify::currently_playing(&*state.spotify.read().await).await;

    let (track, context) = match playing {
        CurrentlyPlaying::Err(err) => {
            return Err(err).context("Get currently playing track");
        },
        CurrentlyPlaying::None(reason) => {
            SpotifyAuthService::suspend_for(&state.app.db, user_id, chrono::Duration::seconds(10))
                .await?;

            return Ok(CheckPlayingForUserResult::None(reason));
        },
        CurrentlyPlaying::Ok(track, context) => (track, context),
    };

    let status = TrackStatusService::get_status(
        &state.app.db,
        &state.user_id,
        &spotify::get_track_id(&track),
    )
    .await;

    match status {
        TrackStatus::Disliked => {
            handle_disliked_track(&state, &track, context.as_ref()).await?;
        },
        TrackStatus::None => {
            let changed = UserService::sync_current_playing(
                &state.app.db,
                &state.user_id,
                &spotify::get_track_id(&track),
            )
            .await?;

            if !changed {
                return Ok(CheckPlayingForUserResult::SkipSame);
            }

            let res = check_bad_words(&state, &track)
                .await
                .context("Check bad words");

            match res {
                Ok(res) => {
                    UserService::increase_stats_query(&state.user_id)
                        .lyrics(
                            1,
                            res.profane as u32,
                            matches!(res.provider, Some(lyrics::Provider::Genius)) as u32,
                            matches!(res.provider, Some(lyrics::Provider::Musixmatch)) as u32,
                        )
                        .exec(&state.app.db)
                        .await?;
                },
                Err(err) => {
                    tracing::error!(
                        err = ?err.anyhow(),
                        track_id = %spotify::get_track_id(&track),
                        track_name = %spotify::create_track_name(&track),
                        "Error occurred on checking bad words",
                    )
                },
            }
        },
        TrackStatus::Ignore => {},
    }

    Ok(CheckPlayingForUserResult::Complete)
}

lazy_static::lazy_static! {
    pub static ref PROCESS_TIME_CHANNEL: (
        broadcast::Sender<CheckPlayingReport>,
        broadcast::Receiver<CheckPlayingReport>
    ) = broadcast::channel(5);
}

#[derive(Clone)]
pub struct CheckPlayingReport {
    pub max_process_time: Duration,
    pub users_process_time: Duration,
    pub users_count: usize,
    pub users_checked: usize,
    pub parallel_count: usize,
}

#[tracing::instrument(skip_all)]
pub async fn check_playing(app_state: &'static state::AppState) {
    utils::tick!(Duration::from_secs(CHECK_INTERVAL), {
        let start = Instant::now();

        let user_ids = match SpotifyAuthService::get_registered(&app_state.db).await {
            Ok(user_ids) => user_ids,
            Err(err) => {
                tracing::error!(err = ?err.anyhow(), "Something went wrong");
                continue;
            },
        };

        let semaphore = Arc::new(Semaphore::new(PARALLEL_CHECKS));
        let user_ids_len = user_ids.len();
        let mut join_handles = Vec::with_capacity(user_ids_len);

        for user_id in user_ids {
            let permit = semaphore
                .clone()
                .acquire_owned()
                .await
                .context("Shouldn't fail")
                .expect("Shouldn't fail");

            join_handles.push(tokio::spawn(async move {
                let user_id = user_id.as_str();
                let res = check_playing_for_user(app_state, user_id).await;
                drop(permit);
                let checked = match res {
                    Err(err) => {
                        tracing::error!(user_id, err = ?err.anyhow(), "Something went wrong");
                        false
                    },
                    Ok(CheckPlayingForUserResult::Complete) => true,
                    _ => false,
                };

                checked
            }));
        }

        let mut users_checked = 0;
        for handle in join_handles {
            if handle.await.expect("Shouldn't fail") {
                users_checked += 1;
            }
        }

        let report = CheckPlayingReport {
            max_process_time: Duration::from_secs(CHECK_INTERVAL),
            users_process_time: start.elapsed(),
            parallel_count: PARALLEL_CHECKS,
            users_count: user_ids_len,
            users_checked,
        };

        PROCESS_TIME_CHANNEL.0.send(report).ok();
    });
}

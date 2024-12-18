use anyhow::{Context as _, bail};
use indoc::formatdoc;
use isolang::Language;
use redis::AsyncCommands as _;
use rustrict::Type;
use teloxide::prelude::*;
use teloxide::types::{ChatId, InlineKeyboardMarkup, ParseMode, ReplyMarkup};

use crate::spotify::ShortTrack;
use crate::state::AppState;
use crate::telegram::inline_buttons::InlineButtons;
use crate::telegram::utils::link_preview_small_top;
use crate::user_service::UserService;
use crate::{lyrics, profanity, state, telegram};

#[derive(Serialize, Deserialize)]
pub struct ProfanityCheckQueueTask {
    track: ShortTrack,
    user_id: String,
}

#[tracing::instrument(
    skip_all,
    fields(
        track_id = track.id(),
        track_name = track.name_with_artists(),
        user_id,
    )
)]
pub async fn queue(
    mut redis: redis::aio::MultiplexedConnection,
    user_id: &str,
    track: &ShortTrack,
) -> anyhow::Result<()> {
    let channel = "rustify:profanity_check".to_string();

    let data = serde_json::to_string(&ProfanityCheckQueueTask {
        track: track.clone(),
        user_id: user_id.into(),
    })?;

    let _: () = redis.lpush(channel, data).await?;

    Ok(())
}

#[tracing::instrument(skip_all)]
pub async fn consume(
    app: &'static AppState,
    mut redis: redis::aio::MultiplexedConnection,
) -> anyhow::Result<()> {
    let channel = "rustify:profanity_check".to_string();

    let message: Option<(String, String)> = redis.brpop(channel, 0.0).await?;

    let Some((_channel, message)) = message else {
        bail!("No message")
    };

    let data: ProfanityCheckQueueTask = serde_json::from_str(&message)?;

    let user_state = app.user_state(&data.user_id).await?;

    let res = check(app, &user_state, &data.track)
        .await
        .context("Check lyrics failed")?;

    UserService::increase_stats_query(user_state.user_id())
        .checked_lyrics(res.profane, res.provider)
        .exec(app.db())
        .await?;

    Ok(())
}

#[derive(Default)]
pub struct CheckBadWordsResult {
    pub skipped: bool,
    pub found: bool,
    pub profane: bool,
    pub provider: Option<lyrics::Provider>,
}

#[tracing::instrument(
    skip_all,
    fields(
        track_id = track.id(),
        track_name = track.name_with_artists(),
    )
)]
pub async fn check(
    app: &'static AppState,
    state: &state::UserState,
    track: &ShortTrack,
) -> anyhow::Result<CheckBadWordsResult> {
    let mut ret = CheckBadWordsResult::default();

    let Some(hit) = app.lyrics().search_for_track(track).await? else {
        return Ok(ret);
    };

    ret.provider = Some(hit.provider());
    ret.found = true;

    if hit.language() != Language::Eng {
        tracing::trace!(language = %hit.language(), provider = %hit.provider(), "Track has non English lyrics");

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
                "<code>{}:</code> {}, <code>[{}]</code>",
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
            r#"
                Current song ({track_name}) probably has bad words (press 'Ignore text 🙈' in case of false positive):

                {bad_lines}

                <a href="{lyrics_link}">{lyrics_link_text}</a>
            "#,
            track_name = track.track_tg_link(),
            bad_lines = bad_lines[0..lines].join("\n"),
            lyrics_link = hit.link(),
            lyrics_link_text = hit.link_text(lines == bad_lines.len()),
        );

        if message.len() <= telegram::MESSAGE_MAX_LEN {
            break message;
        }

        lines -= 1;
    };

    let result: Result<Message, teloxide::RequestError> = app
        .bot()
        .send_message(ChatId(state.user_id().parse()?), message)
        .parse_mode(ParseMode::Html)
        .link_preview_options(link_preview_small_top(track.url()))
        .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(
            #[rustfmt::skip]
            vec![
                vec![InlineButtons::Dislike(track.id().into()).into()],
                vec![InlineButtons::Ignore(track.id().into()).into()],
            ],
        )))
        .send()
        .await;

    crate::telegram::errors::handle_blocked_bot(app, state, result)
        .await
        .map(|_| ret)
}

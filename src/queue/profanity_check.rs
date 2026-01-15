use anyhow::Context as _;
use apalis::prelude::{Data, TaskSink};
use isolang::Language;
use rustrict::Type;
use teloxide::prelude::*;
use teloxide::types::{ChatId, InlineKeyboardMarkup, ParseMode, ReplyMarkup};

use crate::app::App;
use crate::infrastructure::error_handler;
use crate::services::{
    TrackLanguageStatsService,
    UserService,
    UserWordWhitelistService,
    WordStatsService,
};
use crate::spotify::ShortTrack;
use crate::telegram::inline_buttons::InlineButtons;
use crate::telegram::utils::link_preview_small_top;
use crate::user::UserState;
use crate::utils::StringUtils;
use crate::{lyrics, profanity, telegram};

#[derive(Clone, Serialize, Deserialize)]
pub struct ProfanityCheckQueueTask {
    track: ShortTrack,
    user_id: String,
}

#[tracing::instrument(
    skip_all,
    fields(
        track_id = %track.id(),
        track_name = %track.name_with_artists(),
        %user_id,
    )
)]
pub async fn queue(app: &App, user_id: &str, track: &ShortTrack) -> anyhow::Result<()> {
    app.queue_manager()
        .profanity_queue()
        .push(ProfanityCheckQueueTask {
            track: track.clone(),
            user_id: user_id.into(),
        })
        .await?;

    Ok(())
}

#[tracing::instrument(skip_all, fields(user_id = %data.user_id, track_id = %data.track.id()))]
pub async fn consume(data: ProfanityCheckQueueTask, app: Data<&'static App>) -> anyhow::Result<()> {
    let app = *app;

    let user_state = app.user_state(&data.user_id).await;

    let user_state = match user_state {
        Ok(user_state) => user_state,
        Err(mut err) => {
            let res = error_handler::handle(&mut err, app, &data.user_id, "en").await;

            return if res.handled { Ok(()) } else { Err(err) };
        },
    };

    let err_wrap = || async {
        let res = check(app, &user_state, &data.track)
            .await
            .context("Check lyrics failed")?;

        UserService::increase_stats_query(user_state.user_id())
            .checked_lyrics(res.profane, res.provider)
            .exec(app.db())
            .await?;

        Ok::<(), anyhow::Error>(())
    };

    let res = err_wrap().await;

    match res {
        Ok(()) => Ok(()),
        Err(mut err) => {
            let res =
                error_handler::handle(&mut err, app, &data.user_id, user_state.locale()).await;

            if res.handled { Ok(()) } else { Err(err) }
        },
    }
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
        track_id = %track.id(),
        track_name = %track.name_with_artists(),
    )
)]
pub async fn check(
    app: &'static App,
    state: &UserState,
    track: &ShortTrack,
) -> anyhow::Result<CheckBadWordsResult> {
    let mut ret = CheckBadWordsResult::default();

    let Some(hit) = app.lyrics().search_for_track(track).await? else {
        if let Err(err) =
            TrackLanguageStatsService::increase_count(app.db(), None, state.user_id()).await
        {
            tracing::error!(err = ?err, "Error occurred on increasing language stats");
        }
        return Ok(ret);
    };

    ret.provider = Some(hit.provider());
    ret.found = true;

    if let Err(err) =
        TrackLanguageStatsService::increase_count(app.db(), Some(hit.language()), state.user_id())
            .await
    {
        tracing::error!(err = ?err, "Error occurred on increasing language stats");
    }

    if hit.language() != Language::Eng {
        tracing::trace!(language = %hit.language(), provider = %hit.provider(), "Track has non English lyrics");

        ret.skipped = true;
        return Ok(ret);
    }

    let check = profanity::Manager::check(&hit.lyrics());

    if !check.should_trigger() {
        return Ok(ret);
    }

    WordStatsService::increase_check_occurence(app.db(), &check.get_profine_words()).await?;

    let ok_words =
        UserWordWhitelistService::get_ok_words_for_user(app.db(), state.user_id()).await?;

    let bad_lines: Vec<_> = check
        .iter()
        .filter(|profanity::LineResult { typ, .. }| !typ.is(Type::SAFE))
        .filter(|line| {
            let words = line.get_profine_words();

            words.difference(&ok_words).next().is_some()
        })
        .map(|line: &profanity::LineResult| {
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
    let text = loop {
        let message = t!(
            "profanity-check.message",
            locale = state.locale(),
            track_name = track.track_tg_link(),
            bad_lines = bad_lines[0..lines].join("\n"),
            lyrics_link = hit.link().trim(),
            lyrics_link_text = hit.link_text(lines == bad_lines.len()),
        );

        if message.chars_len() <= telegram::MESSAGE_MAX_LEN {
            break message;
        }

        lines -= 1;
    };

    let mut keyboard = vec![
        vec![InlineButtons::Dislike(track.id().into()).into_inline_keyboard_button(state.locale())],
        vec![InlineButtons::Ignore(track.id().into()).into_inline_keyboard_button(state.locale())],
    ];

    if app.ai().is_some() {
        keyboard.push(vec![
            InlineButtons::Analyze(track.id().into()).into_inline_keyboard_button(state.locale()),
        ]);
    }

    let result: Result<Message, teloxide::RequestError> = app
        .bot()
        .send_message(ChatId(state.user_id().parse()?), text)
        .parse_mode(ParseMode::Html)
        .link_preview_options(link_preview_small_top(track.url()))
        .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(
            keyboard,
        )))
        .await;

    match result {
        Ok(_) => Ok(ret),
        Err(err) => {
            let mut err = err.into();
            error_handler::handle(&mut err, app, state.user_id(), state.locale()).await;
            Err(err)
        },
    }
}

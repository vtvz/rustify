use indoc::formatdoc;
use isolang::Language;
use rustrict::Type;
use teloxide::prelude::*;
use teloxide::types::{ChatId, InlineKeyboardMarkup, ParseMode, ReplyMarkup};

use crate::spotify::ShortTrack;
use crate::state::AppState;
use crate::telegram::inline_buttons::InlineButtons;
use crate::{lyrics, profanity, state, telegram};

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
    app_state: &'static AppState,
    state: &state::UserState,
    track: &ShortTrack,
) -> anyhow::Result<CheckBadWordsResult> {
    let mut ret = CheckBadWordsResult::default();

    let Some(hit) = app_state.lyrics().search_for_track(track).await? else {
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
            tracing::info!(line = ?line.line, censored = ?line.censored, typ = %line.typ, "Detected profanity line");

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

    let result: Result<Message, teloxide::RequestError> = app_state
        .bot()
        .send_message(ChatId(state.user_id().parse()?), message)
        .parse_mode(ParseMode::Html)
        .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(
            #[rustfmt::skip]
            vec![
                vec![InlineButtons::Dislike(track.id().into()).into()],
                vec![InlineButtons::Ignore(track.id().into()).into()],
            ],
        )))
        .send()
        .await;

    super::errors::telegram(app_state, state, result)
        .await
        .map(|_| ret)
}

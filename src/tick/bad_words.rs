use indoc::formatdoc;
use isolang::Language;
use rspotify::model::FullTrack;
use rustrict::Type;
use teloxide::prelude::*;
use teloxide::types::{ChatId, InlineKeyboardMarkup, ParseMode, ReplyMarkup};

use crate::errors::GenericResult;
use crate::telegram::inline_buttons::InlineButtons;
use crate::{lyrics, profanity, spotify, state, telegram};

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
        track_id = %spotify::utils::get_track_id(track),
        track_name = %spotify::utils::create_track_name(track),
    )
)]
pub async fn check(
    state: &state::UserState,
    track: &FullTrack,
) -> GenericResult<CheckBadWordsResult> {
    let mut ret = CheckBadWordsResult::default();

    let Some(hit) = state.app.lyrics.search_for_track(track).await? else {
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
            track_name = spotify::utils::create_track_tg_link(track),
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
                vec![InlineButtons::Dislike(spotify::utils::get_track_id(track)).into()],
                vec![InlineButtons::Ignore(spotify::utils::get_track_id(track)).into()],
            ],
        )))
        .send()
        .await;

    super::errors::telegram(state, result).await.map(|_| ret)
}

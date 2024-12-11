use teloxide::types::{LinkPreviewOptions, Message, MessageEntityKind};

pub fn extract_url_from_message(m: &Message) -> Option<url::Url> {
    let entities = m.parse_entities()?;

    let entity = entities
        .iter()
        .find(|entity| entity.kind() == &MessageEntityKind::Url)?;

    url::Url::parse(entity.text()).ok()
}

pub fn link_preview_small_top(url: impl Into<String>) -> LinkPreviewOptions {
    LinkPreviewOptions {
        is_disabled: false,
        url: Some(url.into()),
        prefer_small_media: true,
        prefer_large_media: false,
        show_above_text: true,
    }
}

pub fn link_preview_disabled() -> LinkPreviewOptions {
    LinkPreviewOptions {
        is_disabled: true,
        url: None,
        prefer_small_media: false,
        prefer_large_media: false,
        show_above_text: false,
    }
}

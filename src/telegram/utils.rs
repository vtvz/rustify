use teloxide::types::{Message, MessageEntityKind};

pub fn extract_url_from_message(m: &Message) -> Option<url::Url> {
    let entities = m.parse_entities()?;

    let entity = entities
        .iter()
        .find(|entity| entity.kind() == &MessageEntityKind::Url)?;

    url::Url::parse(entity.text()).ok()
}

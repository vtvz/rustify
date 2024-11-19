use std::collections::HashSet;

use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    // https://github.com/khanhas/spicetify-cli/blob/master/CustomApps/lyrics-plus/Utils.js#L50
    static ref RG_EXTRA_1: Regex = Regex::new(r"\s-\s.*").expect("Should be compilable");
    static ref RG_EXTRA_2: Regex = Regex::new(r"[^\pL_]+").expect("Should be compilable");
    // https://github.com/khanhas/spicetify-cli/blob/master/CustomApps/lyrics-plus/Utils.js#L41
    static ref RG_FEAT_1: Regex =
        Regex::new(r"(?i)-\s+(feat|with).*").expect("Should be compilable");
    static ref RG_FEAT_2: Regex =
        Regex::new(r"(?i)(\(|\[)(feat|with)\.?\s+.*(\)|\])$").expect("Should be compilable");
}

fn remove_extra_info(name: &str) -> String {
    name.replace(&*RG_EXTRA_1, "")
        .replace(&*RG_EXTRA_2, " ")
        .trim()
        .to_owned()
}

fn remove_song_feat(name: &str) -> String {
    name.replace(&*RG_FEAT_1, "")
        .replace(&*RG_FEAT_2, "")
        .trim()
        .to_owned()
}

pub fn get_track_names(name: &str) -> HashSet<String> {
    let no_extra = remove_extra_info(name);
    let names = vec![
        name.to_owned(),
        no_extra.clone(),
        remove_song_feat(name),
        remove_song_feat(&no_extra),
    ];

    HashSet::from_iter(names)
}

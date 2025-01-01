alter table "user"
    add cfg_check_profanity bool default true not null;

alter table "user"
    add cfg_skip_tracks bool default true not null;

alter table "user"
    add cfg_not_english_alert bool default false not null;


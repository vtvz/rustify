create table user_whitelist
(
    id         integer not null
        constraint user_whitelist_pk
            primary key autoincrement,
    user_id    text    not null
        constraint user_whitelist_user_ids
            unique,
    status     text    not null,
    created_at text default (datetime('now', 'localtime')) not null,
    updated_at text default (datetime('now', 'localtime')) not null
);

-- Add migration script here
create table user
(
    id                 text not null
        constraint user_pk
            primary key,
    name               text not null default 'unknown',
    removed_playlists  int  not null default 0,
    removed_collection int  not null default 0,
    status             text          default 'active' not null,
    created_at         text          default (datetime('now', 'localtime')) not null,
    updated_at         text          default (datetime('now', 'localtime')) not null
);

create table "user"
(
  id                 text      not null primary key,
  name               text      not null default 'unknown',
  removed_playlists  bigint    not null default 0,
  removed_collection bigint    not null default 0,
  status             text      not null default 'active',
  created_at         timestamp not null default current_timestamp,
  updated_at         timestamp not null default current_timestamp,
  playing_track      text      not null default '',

  lyrics_checked     bigint not null default 0,
  lyrics_genius      bigint not null default 0,
  lyrics_musixmatch  bigint not null default 0,
  lyrics_profane     bigint not null default 0
);

create unique index user_id_uindex on "user" (id);

create index user_status_index on "user" (status);

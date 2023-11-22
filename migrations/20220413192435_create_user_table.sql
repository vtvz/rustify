create table "user"
(
  id                 text      not null primary key,
  name               text      not null default 'unknown',
  removed_playlists  int       not null default 0,
  removed_collection int       not null default 0,
  status             text      not null default 'active',
  created_at         timestamp not null default current_timestamp,
  updated_at         timestamp not null default current_timestamp,
  playing_track      text      not null default '',

  lyrics_checked     int not null default 0,
  lyrics_genius      int not null default 0,
  lyrics_musixmatch  int not null default 0,
  lyrics_profane     int not null default 0
);

create unique index user_id_uindex on "user" (id);

create index user_status_index on "user" (status);

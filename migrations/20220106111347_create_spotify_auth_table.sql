create table spotify_auth
(
  user_id       text      not null primary key,
  access_token  text      not null default '',
  refresh_token text      not null default '',
  created_at    timestamp not null default current_timestamp,
  updated_at    timestamp not null default current_timestamp,
  expires_at    timestamp,
  suspend_until timestamp not null default current_timestamp
);

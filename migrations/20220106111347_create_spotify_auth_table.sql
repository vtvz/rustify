create table spotify_auth
(
   	user_id text not null constraint spotify_auth_pk primary key,
    access_token  text not null default '',
    refresh_token text not null default '',
    created_at text default (datetime('now', 'localtime')) not null,
    updated_at text default (datetime('now', 'localtime')) not null
);

create unique index spotify_auth_user_id_uindex
	on spotify_auth (user_id);


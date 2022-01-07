create table spotify_auth
(
    telegram_id   text  not null
        constraint spotify_auth_pk
            primary key,
    access_token  text not null,
    refresh_token text not null
);

create unique index spotify_auth_telegram_id_uindex
    on spotify_auth (telegram_id);

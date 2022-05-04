alter table spotify_auth
    add suspend_until text default '2022-01-01 00:00:00' not null;

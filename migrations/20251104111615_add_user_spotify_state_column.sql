alter table "user"
    add spotify_state uuid default uuid_generate_v4() not null;

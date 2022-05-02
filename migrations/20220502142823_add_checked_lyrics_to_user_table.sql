alter table user
    add lyrics_checked int not null default 0;

alter table user
    add lyrics_genius int not null default 0;

alter table user
    add lyrics_musixmatch int not null default 0;

alter table user
    add lyrics_profane int not null default 0;

create table track_language_stats
(
    id         serial primary key,
    user_id    text                                not null
        constraint track_language_stats_user_id_fk
            references "user",
    language   text      default null,
    count      integer   default 0                 not null,
    created_at timestamp default current_timestamp not null,
    updated_at timestamp default current_timestamp not null,
    constraint track_language_stats_user_language_unique
        unique (user_id, language)
);

create index track_language_stats_language_idx on track_language_stats (language);

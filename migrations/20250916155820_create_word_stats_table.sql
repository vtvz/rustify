create table word_stats
(
    id                  serial
        constraint word_stat_pk
            primary key,
    word                text                                not null
        constraint word_stat_word
            unique,
    check_occurrences   integer   default 0                 not null,
    details_occurrences integer   default 0                 not null,
    analyze_occurrences integer   default 0                 not null,
    created_at          timestamp default current_timestamp not null,
    updated_at          timestamp default current_timestamp not null
);

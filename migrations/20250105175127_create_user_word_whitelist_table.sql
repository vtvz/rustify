create table user_word_whitelist
(
    id         serial                              not null
        constraint user_words_whitelist_id_pk
            primary key
        constraint user_words_whitelist_id_unique
            unique,
    user_id    text                                not null
        constraint user_words_whitelist_user_id_fk
            references "user",
    created_at timestamp default current_timestamp not null,
    updated_at timestamp default current_timestamp not null,
    word       text                                not null default '',

    constraint user_words_whitelist_user_id_word_unique
        unique (user_id, word)
);


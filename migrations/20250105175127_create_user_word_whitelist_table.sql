create table user_word_whitelist
(
    id      serial not null
        constraint user_words_whitelist_id_pk
            primary key
        constraint user_words_whitelist_id_unique
            unique,
    user_id text   not null
        constraint user_words_whitelist_user_id_fk
            references "user",
    word    text,
    constraint user_words_whitelist_user_id_word_unique
        unique (user_id, word)
);


create table word_definition
(
    id         serial
        constraint word_definition_pk
            primary key,
    word       text                                not null,
    locale     text                                not null,
    definition text                                not null,
    created_at timestamp default CURRENT_TIMESTAMP not null,
    updated_at timestamp default CURRENT_TIMESTAMP not null,
    constraint word_definition_word_locale
        unique (word, locale)
);

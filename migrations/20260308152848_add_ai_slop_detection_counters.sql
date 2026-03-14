alter table "user"
    add column ai_slop_spotify_ai_blocker bigint not null default 0,
    add column ai_slop_soul_over_ai bigint not null default 0,
    add column ai_slop_shlabs bigint not null default 0;

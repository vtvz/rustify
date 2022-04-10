-- Add migration script here
alter table track_status
    add skips integer default 0;

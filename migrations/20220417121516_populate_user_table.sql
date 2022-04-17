-- Add migration script here
insert into user (id, created_at, updated_at)
select user_id, created_at, updated_at
from spotify_auth;

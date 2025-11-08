update "user" set status = 'spotify_forbidden' where status = 'forbidden';
update "user" set status = 'spotify_token_invalid' where status = 'token_invalid';

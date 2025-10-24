update "user"
set status = 'pending'
where status <> 'pending'
  and id not in (select user_id
                 from spotify_auth
                 where user_id is not null);

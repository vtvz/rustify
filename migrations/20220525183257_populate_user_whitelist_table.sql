insert into user_whitelist (user_id, status, created_at, updated_at)
select id, 'allowed', created_at,  created_at
from user;

create table user_whitelist
(
  id         serial    not null primary key,
  user_id    text      not null unique,
  status     text      not null,
  created_at timestamp not null default current_timestamp,
  updated_at timestamp not null default current_timestamp
);

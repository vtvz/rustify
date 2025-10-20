alter table "user"
    add role text default 'user' not null;

create index user_role_index on "user" (role);

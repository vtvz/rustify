create table track_status
(
	id         serial    primary key,
	user_id    text      not null,
	track_id   text      not null,
  created_at timestamp not null default current_timestamp,
  updated_at timestamp not null default current_timestamp,
	status     text      not null default 'disliked',
	skips      int       not null default 0
);

create index track_status_user_id_index
	on track_status (user_id);

create index track_status_track_id_index
	on track_status (track_id);

create index track_status_status_index
	on track_status (status);

create unique index track_status_user_id_track_id_uindex
	on track_status (user_id, track_id);

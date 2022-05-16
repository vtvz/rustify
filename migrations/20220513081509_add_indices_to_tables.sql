create index track_status_user_id_index
	on track_status (user_id);

create index track_status_track_id_index
	on track_status (track_id);

create index track_status_status_index
	on track_status (status);

create unique index track_status_user_id_track_id_uindex
	on track_status (user_id, track_id);

create unique index user_id_uindex
	on user (id);

create index user_status_index
	on user (status);

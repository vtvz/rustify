create table track_status
(
	id integer not null
		constraint track_status_pk
			primary key autoincrement,
	user_id text not null,
	track_id text not null,
    created_at text default (datetime('now', 'localtime')) not null,
    updated_at text default (datetime('now', 'localtime')) not null,
	status text default 'disliked' not null,
    constraint track_status_ids
	    unique (track_id, user_id)
);

create unique index track_status_id_uindex
	on track_status (id);

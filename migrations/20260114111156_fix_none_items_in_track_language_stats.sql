insert into track_language_stats (user_id, language, count)
select
    user_id,
    'none',
    sum(count)
from track_language_stats
where language is null
group by user_id;

delete from track_language_stats
where language is null;

alter table track_language_stats
    alter column language set not null;

alter table track_language_stats
    alter column language set default 'none';

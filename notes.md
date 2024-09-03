%s/INSERT INTO user /INSERT INTO "user" /
%s/^INSERT INTO _sqlx_migrations /-- INSERT INTO _sqlx_migrations 

---

sqlite3 var/data.db .schema > schema.sql
sqlite3 var/data.db .dump > dump.sql
grep -vx -f schema.sql dump.sql > data.sql


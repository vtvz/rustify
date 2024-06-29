%s/INSERT INTO user /INSERT INTO "user" /

---

sqlite3 some.db .schema > schema.sql
sqlite3 some.db .dump > dump.sql
grep -vx -f schema.sql dump.sql > data.sql

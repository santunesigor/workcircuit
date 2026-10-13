ALTER TABLE tasks ADD COLUMN temporary INTEGER NOT NULL DEFAULT 0 CHECK(temporary IN (0, 1));
PRAGMA user_version = 3;

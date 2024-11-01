-- This file should undo anything in `up.sql`
ALTER TABLE waiting_list DROP COLUMN id;
ALTER TABLE participants DROP COLUMN id;
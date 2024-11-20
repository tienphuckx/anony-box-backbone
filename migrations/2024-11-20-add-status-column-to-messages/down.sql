-- This file should undo anything in `up.sql`
ALTER TABLE messages DROP COLUMN status;
DROP TYPE IF EXISTS MessageStatusType;
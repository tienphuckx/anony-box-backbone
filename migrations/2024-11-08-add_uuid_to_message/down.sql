-- This file should undo anything in `up.sql`
ALTER TABLE messages DROP CONSTRAINT messages_uuid_unique;
ALTER TABLE messages DROP COLUMN message_uuid;
-- This file should undo anything in `up.sql`
ALTER TABLE attachments ALTER COLUMN url DROP NOT NULL;
ALTER TABLE attachments ALTER COLUMN attachment_type DROP NOT NULL;
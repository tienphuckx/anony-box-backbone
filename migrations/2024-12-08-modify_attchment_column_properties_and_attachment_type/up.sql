-- Your SQL goes here
ALTER TYPE AttachmentType ADD VALUE 'BINARY';
ALTER TYPE AttachmentType ADD VALUE 'COMPRESSION';

ALTER TABLE attachments ALTER COLUMN url SET NOT NULL;
ALTER TABLE attachments ALTER COLUMN attachment_type SET NOT NULL;
ALTER TABLE attachments ALTER COLUMN attachment_type SET DEFAULT 'TEXT';

ALTER TABLE attachments DROP CONSTRAINT attachments_message_id_fkey;
ALTER TABLE attachments ADD CONSTRAINT attachments_message_id_fkey FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE;

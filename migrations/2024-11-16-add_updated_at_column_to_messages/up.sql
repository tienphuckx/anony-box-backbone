-- Your SQL goes here
ALTER TABLE messages ADD updated_at timestamp NULL;
COMMENT ON COLUMN messages.updated_at IS 'Store time when updating a message';

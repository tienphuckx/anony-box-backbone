-- Your SQL goes here
ALTER TABLE messages ADD message_uuid uuid NOT NULL;
ALTER TABLE messages ADD CONSTRAINT messages_uuid_unique UNIQUE (message_uuid);

-- Your SQL goes here
CREATE TYPE MessageStatusType AS ENUM (
  'NotSent',
  'Sent',
  'Seen'
);

ALTER TABLE messages ADD status MessageStatusType NOT NULL DEFAULT 'Sent';
COMMENT ON COLUMN messages.status IS 'Store reception status';


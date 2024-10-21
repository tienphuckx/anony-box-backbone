-- This file should undo anything in `up.sql`
DROP TABLE IF EXISTS attachments;
DROP TABLE IF EXISTS messages;
DROP TABLE IF EXISTS waiting_list;
DROP TABLE IF EXISTS participants;
DROP TABLE IF EXISTS groups;
DROP TABLE IF EXISTS users;

DROP TYPE IF EXISTS AttachmentType;
DROP TYPE IF EXISTS MessageType;

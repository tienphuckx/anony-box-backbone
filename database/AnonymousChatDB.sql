CREATE TYPE MessageType AS ENUM (
  'TEXT',
  'ATTACHMENT'
);

CREATE TYPE AttachmentType AS ENUM (
  'TEXT',
  'IMAGE',
  'VIDEO',
  'AUDIO'
);

CREATE TABLE "users" (
  "id" SERIAL PRIMARY KEY,
  "username" varchar(255),
  "user_code" varchar(255),
  "created_at" timestamp
);

CREATE TABLE "groups" (
  "id" SERIAL PRIMARY KEY,
  "name" varchar(255),
  "group_code" varchar(255),
  "user_id" integer NOT NULL,
  "approval_require" bool DEFAULT true,
  "created_at" timestamp,
  "expired_at" timestamp
);

CREATE TABLE "participants" (
  "user_id" integer,
  "group_id" integer,
  PRIMARY KEY ("user_id", "group_id")
);

CREATE TABLE "waiting_list" (
  "user_id" integer,
  "group_id" integer,
  "message" varchar(1000),
  "created_at" timestamp,
  PRIMARY KEY ("user_id", "group_id")
);

CREATE TABLE "messages" (
  "id" SERIAL PRIMARY KEY,
  "content" varchar(1000),
  "message_type" MessageType,
  "created_at" timestamp,
  "user_id" integer NOT NULL,
  "group_id" integer NOT NULL
);

CREATE TABLE "attachments" (
  "id" SERIAL PRIMARY KEY,
  "url" varchar(255),
  "attachment_type" AttachmentType,
  "message_id" integer NOT NULL
);

COMMENT ON COLUMN "groups"."user_id" IS 'Owner of a group';

COMMENT ON TABLE "participants" IS 'User joins a group';

COMMENT ON COLUMN "messages"."user_id" IS 'Owner';

COMMENT ON COLUMN "messages"."group_id" IS 'Message in a group';

COMMENT ON COLUMN "attachments"."message_id" IS 'Attachment of a message';

ALTER TABLE "participants" ADD FOREIGN KEY ("user_id") REFERENCES "users" ("id");

ALTER TABLE "participants" ADD FOREIGN KEY ("group_id") REFERENCES "groups" ("id");

ALTER TABLE "waiting_list" ADD FOREIGN KEY ("user_id") REFERENCES "users" ("id");

ALTER TABLE "waiting_list" ADD FOREIGN KEY ("group_id") REFERENCES "groups" ("id");

ALTER TABLE "groups" ADD FOREIGN KEY ("user_id") REFERENCES "users" ("id");

ALTER TABLE "messages" ADD FOREIGN KEY ("user_id") REFERENCES "users" ("id");

ALTER TABLE "messages" ADD FOREIGN KEY ("group_id") REFERENCES "groups" ("id");

ALTER TABLE "attachments" ADD FOREIGN KEY ("message_id") REFERENCES "messages" ("id");




DROP TYPE IF EXISTS AttachmentType;
DROP TYPE IF EXISTS MessageType;

DROP TABLE IF EXISTS attachments;
DROP TABLE IF EXISTS messages;
DROP TABLE IF EXISTS waiting_list;
DROP TABLE IF EXISTS participants;
DROP TABLE IF EXISTS groups;
DROP TABLE IF EXISTS users;


CREATE TABLE "messages_text" (
                                 "id" SERIAL PRIMARY KEY,
                                 "content" VARCHAR(1000),
                                 "message_type" VARCHAR(255) NOT NULL,  -- Using VARCHAR for message type
                                 "created_at" TIMESTAMP NOT NULL,
                                 "user_id" INTEGER NOT NULL,
                                 "group_id" INTEGER NOT NULL
);

-- Add foreign keys
ALTER TABLE "messages_text" ADD FOREIGN KEY ("user_id") REFERENCES "users" ("id");
ALTER TABLE "messages_text" ADD FOREIGN KEY ("group_id") REFERENCES "groups" ("id");


-- Optional comments for clarity
COMMENT ON COLUMN "messages_text"."user_id" IS 'Owner of the message';
COMMENT ON COLUMN "messages_text"."group_id" IS 'Group the message belongs to';

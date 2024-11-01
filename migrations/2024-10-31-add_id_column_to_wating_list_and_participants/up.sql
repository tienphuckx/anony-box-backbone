-- Your SQL goes here
ALTER TABLE waiting_list ADD id serial not null;
ALTER TABLE waiting_list ADD CONSTRAINT waiting_list_unique UNIQUE (id);

ALTER TABLE participants ADD id serial not null;
ALTER TABLE participants ADD CONSTRAINT participants_unique UNIQUE (id);
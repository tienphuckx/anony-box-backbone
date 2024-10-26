# Backbone Api for Anonychatbox 
An anonymous chatting web

# flow how to create a new table
## create table in ./migrations
     ex. 2024-10-26-create-table-xyz
## create file up.sql and down.sql
    ...
## run
     source .env
     diesel migration run
## config table map for new table in 
    ./src/database/schema.rs
## create model map for new table in ./src/database/models.rs
    ...
## create request payload in ./src/payloads/ 
    ...
## now ready to use from /handles.


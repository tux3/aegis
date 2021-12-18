CREATE TABLE device (
    id integer PRIMARY KEY GENERATED BY DEFAULT AS IDENTITY,
    created_at timestamp NOT NULL,
    name text UNIQUE NOT NULL,
    pubkey text UNIQUE NOT NULL,
    pending boolean NOT NULL
);
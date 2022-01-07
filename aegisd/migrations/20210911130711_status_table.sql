CREATE TABLE device_status(
    dev_id integer REFERENCES device(id) ON DELETE CASCADE NOT NULL,
    updated_at timestamp NOT NULL,
    vt_locked boolean NOT NULL DEFAULT FALSE,
    ssh_locked boolean NOT NULL DEFAULT FALSE
);

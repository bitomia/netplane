CREATE TABLE IF NOT EXISTS users (
    email TEXT PRIMARY KEY NOT NULL,
    role TEXT CHECK( role IN ('admin','user') ) NOT NULL DEFAULT 'user'
);

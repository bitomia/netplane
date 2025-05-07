CREATE TABLE IF NOT EXISTS clients (
    id TEXT PRIMARY KEY NOT NULL,
    client_key TEXT NOT NULL UNIQUE,
    sdn_client_ip TEXT NOT NULL UNIQUE,
    network TEXT NOT NULL,
    netmask TEXT NOT NULL
);

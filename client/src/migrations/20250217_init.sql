CREATE TABLE IF NOT EXISTS clients (
    id TEXT PRIMARY KEY NOT NULL,
    pub_key TEXT,
    sdn_client_ip TEXT NOT NULL UNIQUE,
    network TEXT NOT NULL,
    netmask TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS auth_links (
    id TEXT PRIMARY KEY NOT NULL,
    client_id TEXT,
    used BOOLEAN DEFAULT false,
    FOREIGN KEY (client_id) REFERENCES clients(id)
);

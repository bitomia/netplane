CREATE TABLE new_clients (
    id TEXT PRIMARY KEY NOT NULL,
    pub_key TEXT,
    sdn_client_ip TEXT NOT NULL UNIQUE,
    network TEXT NOT NULL,
    netmask TEXT NOT NULL,
    hostname TEXT UNIQUE
);

INSERT INTO new_clients (id, pub_key, sdn_client_ip, network, netmask)
SELECT id, pub_key, sdn_client_ip, network, netmask FROM clients;

DROP TABLE clients;

ALTER TABLE new_clients RENAME TO clients;


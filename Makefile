.PHONY: client server admin

all: client server admin

client:
	cargo build -p client

server: reticuladb
	cargo build -p server

admin:
	cargo build -p admin

reticuladb:
	sqlx database create
	sqlx migrate run --source ./server/src/migrations

clean:
	cargo clean

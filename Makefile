.PHONY: client server

all: client server

client:
	cargo build -p client

server: reticuladb
	cargo build -p server

reticuladb:
	sqlx database create
	sqlx migrate run --source ./server/src/migrations

clean:
	cargo clean

.PHONY: client server admin

#all: client server admin
all: client server

client:
	cargo build -p client

server: reticuladb
	cargo build -p server

# admin:
# 	cargo build -p server --bin admin

reticuladb:
	sqlx database create
	sqlx migrate run --source ./server/src/migrations

clean:
	cargo clean

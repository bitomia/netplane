all: client server

client:
	cargo build --bin client --features=client

server:
	cargo build --bin server

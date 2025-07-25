.PHONY: client server

all: client server

client: target/debug/netplane
	cargo build -p netplane_client

server: netplanedb target/debug/netplane-server
	cargo build -p netplane_server

netplanedb:
	sqlx database create
	sqlx migrate run --source ./crates/server/src/migrations

webapp:
	cd web; pnpm install --frozen-lockfile; pnpm run build; cd -

release: netplanedb target/release/netplane target/x86_64-unknown-linux-musl/release/netplane-server
	cargo build -p netplane_server --release --target x86_64-unknown-linux-musl
	cargo build -p netplane_client --release

docker: netplanedb target/x86_64-unknown-linux-musl/release/netplane-server
	cargo build -p netplane_server --release --target x86_64-unknown-linux-musl
	docker build -t ghcr.io/bitomia/netplane-server -f Dockerfile.server .

clean:
	cargo clean

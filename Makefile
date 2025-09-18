.PHONY: client server

DEFAULT_NETPLANE_DB=sqlite://netplane.db

all: client server

client:
	cargo build -p netplane_client

server: netplanedb
	cargo build -p netplane_server

netplanedb:
	sqlx database create -D $(DEFAULT_NETPLANE_DB)
	sqlx migrate run --source ./crates/server/src/migrations -D $(DEFAULT_NETPLANE_DB)
	cargo sqlx prepare --workspace -D $(DEFAULT_NETPLANE_DB)

webapp:
	cd web; pnpm install --frozen-lockfile; pnpm run build; cd -

release: netplanedb target/release/netplane target/x86_64-unknown-linux-musl/release/netplane-server
	cargo build -p netplane_server --release --target x86_64-unknown-linux-musl
	cargo build -p netplane_client --release

docker: netplanedb
	cargo build -p netplane_server --release --target x86_64-unknown-linux-musl
	docker build -t ghcr.io/bitomia/netplane-server -f Dockerfile.server .

clean:
	cargo clean

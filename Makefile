.PHONY: client server

DEFAULT_NETPLANE_DB=sqlite://netplane.db

all: client server

client:
	cargo build -p netplane_client

server: netplanedb
	cargo build -p netplane_server

android_debug:
	cargo ndk -t arm64-v8a -t armeabi-v7a build -p netplane_client --lib

android_emu_debug:
	cargo ndk -t x86_64 -t x86 build -p netplane_client --lib

android_release:
	cargo ndk -t arm64-v8a -t armeabi-v7a build -p netplane_client --lib --release

android_emu_release:
	cargo ndk -t x86_64 -t x86 build -p netplane_client --lib --release

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

DEFAULT_NETPLANE_DB=sqlite://netplane.db

.PHONY: all
all: webapp crates

.PHONE: crates
crates: client server

.PHONY: client
client:
	cargo build -p netplane_client

.PHONY: server
server: db-prepare
	cargo build -p netplane_server

.PHONY: android-debug
android-debug:
	cargo ndk -t arm64-v8a -t armeabi-v7a build -p netplane_client --lib

.PHONY: android-emu-debug
android-emu-debug:
	cargo ndk -t x86_64 -t x86 build -p netplane_client --lib

.PHONY: android-release
android-release:
	cargo ndk -t arm64-v8a -t armeabi-v7a build -p netplane_client --lib --release

.PHONY: android-emu-release
android-emu-release:
	cargo ndk -t x86_64 -t x86 build -p netplane_client --lib --release

.PHONY: android
android: android-debug android-emu-debug android-release android-emu-release

.PHONY: db-prepare
db-prepare:
	sqlx database create -D $(DEFAULT_NETPLANE_DB)
	sqlx migrate run --source ./crates/server/src/migrations -D $(DEFAULT_NETPLANE_DB)
	cargo sqlx prepare --workspace -D $(DEFAULT_NETPLANE_DB) -- -p netplane_server

.PHONY: webapp
webapp:
	bun install --frozen-lockfile --cwd web
	bun run --cwd web build

.PHONY: release
release:
	cargo build -p netplane_server --release --target x86_64-unknown-linux-musl
	cargo build -p netplane_client --release

.PHONY: docker
docker:
	cargo build -p netplane_server --release --target x86_64-unknown-linux-musl
	docker build -t ghcr.io/bitomia/netplane-server -f Dockerfile.server .

.PHONY: verify-fmt
verify-fmt:
	cargo fmt -- --check

.PHONY: verify-lint
verify-lint:
	cargo clippy --all-targets --all-features

.PHONY: verify
verify: verify-fmt verify-lint

clean:
	cargo clean

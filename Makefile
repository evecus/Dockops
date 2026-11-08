.PHONY: all build build-web build-rust run clean test musl-amd64 musl-arm64 deps

VERSION ?= dev
BINARY  := dockops

all: build

# ===== Development =====
dev: build
	./target/release/$(BINARY) --http 9080

build-web:
	@echo "Building frontend..."
	cd web && npm ci && npm run build

build-rust:
	@echo "Building rust binary..."
	DOCKOPS_VERSION=$(VERSION) cargo build --release
	@echo "Binary: ./target/release/$(BINARY)"

build: build-web build-rust

run: build
	./target/release/$(BINARY)

# ===== musl cross builds (same commands as the CI workflows) =====
musl-amd64: build-web
	cross build --release --locked --target x86_64-unknown-linux-musl
	mkdir -p dist && cp target/x86_64-unknown-linux-musl/release/$(BINARY) dist/$(BINARY)-linux-amd64-musl

musl-arm64: build-web
	cross build --release --locked --target aarch64-unknown-linux-musl
	mkdir -p dist && cp target/aarch64-unknown-linux-musl/release/$(BINARY) dist/$(BINARY)-linux-arm64-musl

# ===== Test & deps =====
test:
	cargo test

deps:
	cargo fetch
	cd web && npm ci

clean:
	cargo clean
	rm -rf dist/ web/dist/

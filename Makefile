.PHONY: build dev test clean check lint fmt

build:
	cd src-tauri && cargo build

dev:
	cd src-tauri && cargo build

test:
	cd src-tauri && cargo test

check:
	cd src-tauri && cargo check

clean:
	cd src-tauri && cargo clean

lint:
	cd src-tauri && cargo clippy -- -D warnings

fmt:
	cd src-tauri && cargo fmt

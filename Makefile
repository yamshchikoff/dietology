.PHONY: build dev test test-all test-web clean check lint fmt

build:
	cd src-tauri && cargo build

dev:
	cd src-tauri && cargo build

test:
	cd src-tauri && cargo test --lib --test llm_client_tests --test llm_types_tests --test llm_session_tests --test data_loader_tests

test-all:
	cd src-tauri && cargo test

test-web:
	cd web/test && npm install --silent 2>/dev/null; cd ../.. && node web/test/browser_test.mjs

check:
	cd src-tauri && cargo check

clean:
	cd src-tauri && cargo clean

lint:
	cd src-tauri && cargo clippy -- -D warnings

fmt:
	cd src-tauri && cargo fmt

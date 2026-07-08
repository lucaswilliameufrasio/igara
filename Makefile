.PHONY: format lint lint-format test test-nocapture test-watch build build-release clean

format:
	cargo fmt --all

lint:
	cargo clippy --all-targets --all-features -- -D warnings

lint-format: format lint

test:
	cargo test

test-nocapture:
	cargo test -- --nocapture

test-watch:
	cargo watch -x test

test-all:
	cargo test --all-features

build:
	cargo build

build-release:
	cargo build --release

clean:
	cargo clean

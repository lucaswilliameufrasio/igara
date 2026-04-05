.PHONY: format lint lint-format test build

format:
	cargo fmt --all

lint:
	cargo clippy --all-targets --all-features -- -D warnings

lint-format: format lint

test:
	cargo test

build:
	cargo build

.PHONY: setup format lint lint-format test test-nocapture test-watch test-all build build-release clean

setup:
	cargo install cargo-nextest --locked
	cargo install cargo-watch --locked
	cargo install cargo-binstall --locked
	cargo binstall --no-confirm cargo-tarpaulin cargo-llvm-cov cargo-bloat 2>/dev/null || true

format:
	cargo fmt --all

lint:
	cargo clippy --all-targets --all-features -- -D warnings

lint-format: format lint

test:
	cargo nextest run

test-nocapture:
	cargo nextest run -- --nocapture

test-watch:
	cargo watch -x "nextest run"

test-all:
	cargo nextest run --all-features

build:
	cargo build

build-release:
	cargo build --release

clean:
	cargo clean

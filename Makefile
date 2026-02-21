.PHONY: all build build-debug test test-unit test-integration lint lint-fmt lint-clippy bench clean install

all: build

build:
	cargo build --release

build-debug:
	cargo build

test: test-unit test-integration

test-unit:
	cargo test --lib

test-integration:
	cargo test --tests

lint: lint-fmt lint-clippy

lint-fmt:
	cargo fmt --check

lint-clippy:
	cargo clippy -- -D warnings

bench:
	cargo bench

clean:
	cargo clean

install:
	cargo install --path .

.PHONY: test fmt clippy

# Run everything a PR must pass: format, lints, unit tests.
test:
	./scripts/test.sh

fmt:
	cargo fmt --all

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

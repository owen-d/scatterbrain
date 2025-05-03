.PHONY: dd

dd:
	export RUSTFLAGS="-D warnings"; \
	cargo check && cargo test && cargo doc --no-deps 
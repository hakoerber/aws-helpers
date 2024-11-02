.PHONY: docs
docs:
	cargo watch -- cargo doc

.PHONY: test
test:
	@cargo test --workspace --color=always

.PHONY: lint
lint:
	@cargo clippy --workspace --tests --color=always

.PHONY: fmt
fmt:
	cargo fmt
	find -name '*.md' | xargs prettier --print-width 80 --prose-wrap always --write
	find -name '*.toml' | xargs taplo format

.PHONY: build-static
build-static:
	cargo build --target x86_64-unknown-linux-musl --no-default-features --release --workspace

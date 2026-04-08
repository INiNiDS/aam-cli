.PHONY: credits fmt lint test check all

# Generate CREDITS.html from dependencies
credits:
	cargo about generate about.hbs -o CREDITS.html

# Format code
fmt:
	cargo fmt

# Run clippy lints
lint:
	cargo clippy -- -D warnings

# Run tests
test:
	cargo test

# Run all checks
check: fmt lint test

# Generate credits and run all checks
all: credits check

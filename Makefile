.PHONY: test test-infra test-infra-down

# Start Postgres, Redis, LocalStack and run migrations
test-infra:
	docker compose up -d db redis localstack migrate

# Stop test infrastructure
test-infra-down:
	docker compose down

# Run all integration tests (requires test-infra)
test:
	cargo test --test '*' -- --test-threads=1

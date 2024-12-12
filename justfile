set dotenv-load

migrate:
  cd ./db && goose postgres "$DATABASE_URL" up

generate:
  cd ./internal/postgres/ && sqlc generate

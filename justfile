set dotenv-load

migrate:
  cd ./migrations/ && goose postgres $DATABASE_URL up


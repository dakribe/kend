-- +goose Up
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

create table users (
  id uuid PRIMARY KEY DEFAULT uuid_generate_v4 (),
  email VARCHAR(255) NOT NULL UNIQUE,
  first_name VARCHAR(100),
  created_at TIMESTAMP
  WITH
    TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP
  WITH
    TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- +goose Down
drop table user;

-- +goose Up
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS "users" (
  id UUID DEFAULT uuid_generate_v4 () PRIMARY KEY,
  email VARCHAR UNIQUE NOT NULL,
  first_name VARCHAR
);

CREATE TABLE IF NOT EXISTS sessions (
  id UUID DEFAULT uuid_generate_v4 () PRIMARY KEY,
  user_id UUID REFERENCES users (id) NOT NULL,
  expires_at TIMESTAMP NOT NULL
);

CREATE TABLE IF NOT EXISTS applications (
  id UUID DEFAULT uuid_generate_v4 () PRIMARY KEY,
  title VARCHAR NOT NULL,
  company VARCHAR NOT NULL,
  created_at TIMESTAMP DEFAULT now ()
);

-- +goose Down
DROP DATABASE IF EXISTS applications;

DROP DATABASE IF EXISTS sessions;

DROP DATABASE IF EXISTS "users";

DROP EXTENSION IF EXISTS "uuid-ossp"

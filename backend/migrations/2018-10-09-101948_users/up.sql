-- Your SQL goes here
CREATE TABLE users (
  email VARCHAR(100) NOT NULL UNIQUE PRIMARY KEY,
  hash VARCHAR(122), --argon hash
  created_at TIMESTAMP NOT NULL,
  oidc_subject VARCHAR(255) UNIQUE
);

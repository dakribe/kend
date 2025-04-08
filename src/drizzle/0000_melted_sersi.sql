CREATE TABLE "applications" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"client_id" bigserial NOT NULL,
	"title" varchar NOT NULL,
	"company" varchar NOT NULL
);

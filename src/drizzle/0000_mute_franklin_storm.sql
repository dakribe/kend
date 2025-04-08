CREATE TABLE "applications" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"title" varchar NOT NULL,
	"company" varchar NOT NULL
);
--> statement-breakpoint
CREATE TABLE "version" (
	"id" integer DEFAULT 0 NOT NULL
);

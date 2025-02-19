ALTER TABLE "applications" ADD PRIMARY KEY ("id");--> statement-breakpoint
ALTER TABLE "applications" ALTER COLUMN "id" SET DEFAULT gen_random_uuid();--> statement-breakpoint
ALTER TABLE "applications" ALTER COLUMN "id" SET NOT NULL;--> statement-breakpoint
ALTER TABLE "applications" ADD COLUMN "title" varchar NOT NULL;--> statement-breakpoint
ALTER TABLE "applications" ADD COLUMN "company" varchar NOT NULL;
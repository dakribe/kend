import { Schema } from "effect";

export class ApplicationInsert extends Schema.Class<ApplicationInsert>(
	"ApplicationInsert",
)({
	title: Schema.NonEmptyString,
	company: Schema.NonEmptyString,
}) {}

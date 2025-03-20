import { Schema } from "effect";

export class ApplicationInsert extends Schema.Class<ApplicationInsert>(
	"ApplicationInsert",
)({
	title: Schema.NonEmptyString,
	company: Schema.NonEmptyString,
}) {}

export class ApplicationSelect extends Schema.Class<ApplicationSelect>(
	"ApplicationSelect",
)({
	id: Schema.Int,
	company: Schema.NonEmptyString,
	title: Schema.NonEmptyString,
}) {}

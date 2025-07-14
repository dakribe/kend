import { action, useAction } from "@solidjs/router";
import { db } from "~/drizzle";
import { application, InsertApplication } from "./sql";
import { createForm, SubmitHandler } from "@modular-forms/solid";

const addApplicationAction = action(async (values: InsertApplication) => {
	"use server";

	await db
		.insert(application)
		.values({ company: values.company, title: values.title })
		.returning();
}, "add-application");

export function CreateForm() {
	const create = useAction(addApplicationAction);
	const [_, { Form, Field }] = createForm<InsertApplication>();

	const handleSubmit: SubmitHandler<InsertApplication> = (values, e) => {
		e.preventDefault();
		create(values);
	};
	return (
		<div>
			<Form onSubmit={handleSubmit}>
				<Field name="company">
					{(field, props) => (
						<>
							<input {...props} type="text" placeholder="Company" />
							{field.error && <div>{field.error}</div>}
						</>
					)}
				</Field>
				<Field name="title">
					{(field, props) => (
						<>
							<input {...props} type="text" placeholder="title" />
							{field.error && <div>{field.error}</div>}
						</>
					)}
				</Field>
				<button type="submit">Create</button>
			</Form>
		</div>
	);
}

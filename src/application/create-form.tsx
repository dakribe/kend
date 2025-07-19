import { addApplication } from "./application";

export function CreateForm() {
	return (
		<form action={addApplication} method="post">
			<input name="title" placeholder="title" type="text" />
			<input name="company" placeholder="company" type="text" />
			<button type="submit">Create</button>
		</form>
	);
}

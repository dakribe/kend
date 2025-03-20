import { Pglite } from "@/services/pglite";
import { RuntimeClient } from "@/services/runtime-client";
import { Effect } from "effect";
import { Button } from "./ui/button";
import { useState } from "react";

export function CreateApplication() {
	const [company, setCompany] = useState("");
	const [title, setTitle] = useState("");

	const handleSubmit = (e: React.FormEvent) => {
		e.preventDefault();
		RuntimeClient.runPromise(
			Effect.gen(function* () {
				const api = yield* Pglite;
				yield* api.createApplication({
					company,
					title,
				});
			}),
		);
	};

	return (
		<div>
			<form onSubmit={(e) => handleSubmit(e)}>
				<input
					type="text"
					onInput={(e) => setCompany(e.currentTarget.value)}
					value={company}
					placeholder="company"
				/>
				<input
					type="text"
					onInput={(e) => setTitle(e.currentTarget.value)}
					value={title}
					placeholder="title"
				/>
				<Button type="submit">Create</Button>
			</form>
		</div>
	);
}

import { Button } from "@/components/ui/button";
import { createApplication, db } from "@/db";
import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { useLiveQuery } from "dexie-react-hooks";

export const Route = createFileRoute("/")({
	component: Index,
});

function Index() {
	const [title, setTitle] = useState("");
	const [company, setCompany] = useState("");

	const handleAdd = () => {
		createApplication(title, company);
	};

	const applications = useLiveQuery(() => db.applications.toArray());

	return (
		<div className="p-2">
			<h3>Welcome Home!</h3>
			<input
				type="text"
				onInput={(e) => setTitle(e.currentTarget.value)}
				placeholder="title"
			/>
			<input
				type="text"
				onInput={(e) => setCompany(e.currentTarget.value)}
				placeholder="company"
			/>
			<Button onClick={() => handleAdd()}>Add</Button>
			<ul>
				{applications?.map((app) => (
					<li key={app.id}>
						{app.company}, {app.title}
					</li>
				))}
			</ul>
		</div>
	);
}

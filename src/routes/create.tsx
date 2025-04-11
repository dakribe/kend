import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { createApplication } from "@/pglite/application";
import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { toast } from "sonner";

export const Route = createFileRoute("/create")({
	component: RouteComponent,
});

function RouteComponent() {
	const [title, setTitle] = useState("");
	const [company, setCompany] = useState("");

	const handleSubmit = (e: React.FormEvent) => {
		e.preventDefault();
		createApplication({ title, company });
		toast(`Created Application: ${company}`);
	};

	return (
		<div>
			<Input
				placeholder="Title"
				onInput={(e) => setTitle(e.currentTarget.value)}
			/>
			<Input
				placeholder="Company"
				onInput={(e) => setCompany(e.currentTarget.value)}
			/>
			<Button onClick={(e) => handleSubmit(e)}>Create</Button>
		</div>
	);
}

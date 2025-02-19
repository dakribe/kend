import { useParams } from "@solidjs/router";
import { db } from "../drizzle/db";
import { eq } from "drizzle-orm";
import { applications } from "../drizzle/schema";
import { createResource } from "solid-js";

async function getApplication(id: string) {
	const [res] = await db
		.select()
		.from(applications)
		.where(eq(applications.id, id));

	return res;
}

export function Application() {
	const params = useParams();
	const [application] = createResource(() => params.id, getApplication);
	return <div>{application()?.title}</div>;
}

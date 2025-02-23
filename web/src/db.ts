import Dexie, { type EntityTable } from "dexie";

export interface Application {
	id: string;
	title: string;
	company: string;
}

export const db = new Dexie("db") as Dexie & {
	applications: EntityTable<Application, "id">;
};

db.version(1).stores({
	applications: "id, title, company",
});

export function createApplication(title: string, company: string) {
	const id = crypto.randomUUID();
	db.applications.add({
		id,
		title,
		company,
	});
}

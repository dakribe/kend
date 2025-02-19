import { onMount } from "solid-js";
import solidLogo from "./assets/solid.svg";
import viteLogo from "/vite.svg";
import "./App.css";
import { migrate } from "./drizzle/migrate";
import { db } from "./drizzle/db";
import { applications } from "./drizzle/schema";

function App() {
	onMount(async () => {
		await migrate();
		const id = crypto.randomUUID();
		await db.insert(applications).values({ id });
		const result = await db.select().from(applications);
		console.log(result);
	});

	return (
		<>
			<div>
				<a href="https://vite.dev" target="_blank">
					<img src={viteLogo} class="logo" alt="Vite logo" />
				</a>
				<a href="https://solidjs.com" target="_blank">
					<img src={solidLogo} class="logo solid" alt="Solid logo" />
				</a>
			</div>
			<h1>Vite + Solid</h1>
			<p class="read-the-docs">
				Click on the Vite and Solid logos to learn more
			</p>
		</>
	);
}

export default App;

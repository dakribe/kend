import { onMount } from "solid-js";
import "./App.css";
import { migrate } from "./drizzle/migrate";
import { Route, Router } from "@solidjs/router";
import { Root } from "./pages/root";
import { Application } from "./pages/application";

function App() {
	onMount(async () => {
		await migrate();
		console.log("Migration ran");
	});

	return (
		<Router>
			<Route path="/" component={Root} />
			<Route path="/:id" component={Application} />
		</Router>
	);
}

export default App;

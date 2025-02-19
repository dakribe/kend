import { useParams } from "@solidjs/router";

export function Application() {
	const params = useParams();
	return <div>Application {params.id}</div>;
}

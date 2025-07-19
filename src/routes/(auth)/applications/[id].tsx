import { createAsync, RouteDefinition, useParams } from "@solidjs/router";
import { getApplication } from "~/application/application";
import { getUser } from "~/user/user";

export const route: RouteDefinition = {
	preload: () => getUser(),
};

export default function Application() {
	const params = useParams();
	const application = createAsync(() => getApplication(params.id));

	return (
		<div>
			<p>{application()?.title}</p>
			<p>{application()?.company}</p>
		</div>
	);
}

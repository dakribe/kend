import { Title } from "@solidjs/meta";

export default function Home() {
	return (
		<main>
			<Title>Hello World</Title>
			<div>
				<a href="/login/google" rel="external">
					Sign in with Google
				</a>
			</div>
		</main>
	);
}

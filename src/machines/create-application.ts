import { ActorRefFrom, assign, fromPromise, setup } from "xstate";
import { textFieldActor } from "./text-field";
import { RuntimeClient } from "@/services/runtime-client";
import { Effect } from "effect";
import { Pglite } from "@/services/pglite";

interface Context {
	company: ActorRefFrom<typeof textFieldActor>;
	title: ActorRefFrom<typeof textFieldActor>;
	submitError: string | null;
}

export const machine = setup({
	types: {
		context: {} as Context,
		events: {} as { type: "application.create" },
	},
	actors: {
		createApplication: fromPromise(
			({ input }: { input: Omit<Context, "submitError"> }) =>
				RuntimeClient.runPromise(
					Effect.gen(function* () {
						const api = yield* Pglite;

						yield* api.createApplication({
							company: input.company.getSnapshot().context.value,
							title: input.title.getSnapshot().context.value,
						});
					}),
				),
		),
	},
}).createMachine({
	id: "application-create",
	context: ({ spawn }) => ({
		title: spawn(textFieldActor),
		company: spawn(textFieldActor),
		submitError: null,
	}),
	initial: "Idle",
	states: {
		Idle: {
			on: {
				"application.create": {
					target: "CreatingApplication",
					actions: assign({ submitError: null }),
				},
			},
		},
		CreatingApplication: {
			invoke: {
				src: "createApplication",
				input: ({ context }) => ({
					company: context.company,
					title: context.title,
				}),
				onError: {
					target: "Idle",
					actions: assign(({ event }) => ({
						submitError:
							event.error instanceof Error
								? event.error.message
								: "Unknown Error",
					})),
				},
				onDone: { target: "Created" },
			},
		},
		Created: { after: { 5000: "Idle" } },
	},
});

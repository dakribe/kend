import { clsx, type ClassValue } from "clsx";
import { Effect, pipe, Array } from "effect";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
	return twMerge(clsx(inputs));
}

export const singleResult = <A, E>(orFail: () => E) =>
	Effect.flatMap((results: A[]) =>
		pipe(results, Array.head, Effect.mapError(orFail)),
	);

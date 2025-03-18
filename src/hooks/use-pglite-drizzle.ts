import { Context } from "effect";
import { createContext, useContext } from "react";
import { Pglite } from "../services/pglite";

export const PgliteDrizzleContext = createContext<
	Context.Tag.Service<typeof Pglite>["orm"] | null
>(null);

export const usePgliteDrizzle = () => {
	const context = useContext(PgliteDrizzleContext);
	if (!context) {
		throw new Error(
			"usePgliteDrizzle must be used within PgliteDrizzleProvider",
		);
	}
};

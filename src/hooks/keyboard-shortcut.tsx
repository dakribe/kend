import { useNavigate } from "@tanstack/react-router";
import { useEffect } from "react";

export function useKeyboardShortcuts() {
	const navigate = useNavigate();

	useEffect(() => {
		let keySequence = "";
		let timeout: number;

		const handleKeyDown = (e: KeyboardEvent) => {
			const target = e.target as HTMLElement;
			if (
				["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName) ||
				target.contentEditable === "true"
			) {
				return;
			}

			keySequence += e.key.toLowerCase();

			if (keySequence.includes("gd")) {
				navigate({ to: "/dashboard" });
				keySequence = "";
				return;
			}

			if (keySequence.includes("ga")) {
				navigate({ to: "/applications" });
				keySequence = "";
				return;
			}

			window.clearTimeout(timeout);
			timeout = window.setTimeout(() => {
				keySequence = "";
			}, 1000);
		};

		document.addEventListener("keydown", handleKeyDown);

		return () => {
			document.removeEventListener("keydown", handleKeyDown);
			window.clearTimeout(timeout);
		};
	}, [navigate]);
}

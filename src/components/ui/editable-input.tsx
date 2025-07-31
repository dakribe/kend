import { MutationFunction, useMutation } from "@tanstack/react-query";
import { useState } from "react";

interface Props {
	mutationFn: MutationFunction;
	initialValue: string;
	id: string;
	className?: string;
}

export function EditableInput({
	initialValue,
	mutationFn,
	id,
	className,
}: Props) {
	const [value, setValue] = useState(initialValue);
	const [isEditing, setIsEditing] = useState(false);

	const mutation = useMutation({
		mutationFn,
		onSuccess: () => setIsEditing(false),
		onError: () => setValue(initialValue),
	});

	const handleSave = () => {
		if (value !== initialValue) {
			mutation.mutateAsync({ id, value });
		} else {
			setIsEditing(false);
		}
	};

	const handleKeyDown = (e: React.KeyboardEvent) => {
		if (e.key === "Enter") {
			handleSave();
		}
		if (e.key === "Escape") {
			setValue(initialValue);
			setIsEditing(false);
		}
	};

	return (
		<div>
			{isEditing ? (
				<input
					type="text"
					value={value}
					onInput={(e) => setValue(e.currentTarget.value)}
					onBlur={handleSave}
					onKeyDown={handleKeyDown}
					autoFocus
					className={`focus:outline-none ${className}`}
				/>
			) : (
				<div
					onClick={() => setIsEditing(true)}
					className={`cursor-text ${className}`}
				>
					{value}
				</div>
			)}
		</div>
	);
}

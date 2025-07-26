import { Link } from "@tanstack/react-router";
import { Bookmark, Home, List, Plus, Settings } from "lucide-react";

interface Props {
	setOpen: React.Dispatch<React.SetStateAction<boolean>>;
}

export function Navbar({ setOpen }: Props) {
	return (
		<div className="fixed rounded-3xl bottom-10 left-1/2 transform -translate-x-1/2 border h-12 flex items-center p-2">
			<Link to="/dashboard" className="hover:cursor-default">
				<div className="hover:bg-accent rounded-full p-2">
					<Home className="size-4" />
				</div>
			</Link>
			<Link to="/applications" className="hover:cursor-default">
				<div className="hover:bg-accent rounded-full p-2">
					<List className="size-4" />
				</div>
			</Link>
			<Link to="/applications" className="hover:cursor-default">
				<div className="hover:bg-accent rounded-full p-2">
					<Bookmark className="size-4" />
				</div>
			</Link>
			<Link to="/applications" className="hover:cursor-default">
				<div className="hover:bg-accent rounded-full p-2">
					<Settings className="size-4" />
				</div>
			</Link>
			<div
				onClick={() => setOpen(true)}
				className="hover:bg-accent rounded-full p-2"
			>
				<Plus className="size-4" />
			</div>
		</div>
	);
}

import { Link } from "@tanstack/react-router";
import { Home, List, LucideIcon, Plus } from "lucide-react";

interface Props {
	setOpen: React.Dispatch<React.SetStateAction<boolean>>;
}

type NavItem = {
	icon: LucideIcon;
} & ({ href: string; onClick?: never } | { onClick: () => void; href?: never });

export function Navbar({ setOpen }: Props) {
	const navItems: NavItem[] = [
		{ icon: Home, href: "/dashboard" },
		{ icon: List, href: "/applications" },
		{ icon: Plus, onClick: () => setOpen(true) },
	];

	return (
		<div className="fixed rounded-3xl bottom-10 left-1/2 transform -translate-x-1/2 border h-12 flex items-center p-2 gap-1">
			{navItems.map((item, index) =>
				item.href ? (
					<Link
						key={index}
						to={item.href}
						activeProps={{
							className: "bg-accent rounded-full",
						}}
					>
						<div className="hover:bg-accent rounded-full p-2 cursor-default">
							<item.icon className="size-4" />
						</div>
					</Link>
				) : (
					<div
						key={index}
						onClick={item.onClick}
						className="hover:bg-accent rounded-full p-2"
					>
						<item.icon className="size-4" />
					</div>
				),
			)}
		</div>
	);
}

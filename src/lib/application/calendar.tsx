import { ResponsiveCalendar } from "@nivo/calendar";
import { useQuery } from "@tanstack/react-query";
import { getCalendarData } from ".";
import { Card, CardDescription, CardHeader } from "@/components/ui/card";

export function Calendar() {
	const { data, isLoading, isError } = useQuery({
		queryKey: ["calendar_data"],
		queryFn: getCalendarData,
	});

	if (isLoading) {
		return <div className="text-muted-foreground">Loading...</div>;
	}

	if (isError) {
		return <div className="text-destructive">Error loading calendar data</div>;
	}

	return (
		<Card className="w-3/4 h-64">
			<CardHeader>
				<CardDescription>Activity</CardDescription>
			</CardHeader>
			<ResponsiveCalendar
				data={data || []}
				from="2025-01-02"
				to={"2025-12-31"}
				emptyColor="oklch(0.3037 0.0726 281.1738)"
				colors={["#22c55e", "#16a34a", "#15803d", "#166534"]}
				margin={{ top: 10, right: 40, bottom: 10, left: 40 }}
				dayBorderWidth={2}
				daySpacing={5}
				dayBorderColor="hsl(var(--background))"
				monthBorderColor="hsl(var(--border))"
				yearLegendOffset={6}
				monthLegendOffset={6}
				theme={{
					background: "transparent",
					text: {
						fill: "hsl(var(--foreground))",
						fontSize: 12,
					},
					labels: {
						text: {
							fill: "oklch(0.6324 0.0388 285.2389)",
						},
					},
					legends: {
						text: {
							fill: "bg-background",
						},
					},
				}}
				tooltip={({ day, value, color }) => (
					<div className="bg-popover text-popover-foreground px-3 py-2 rounded-md shadow-lg text-sm border border-border">
						<div className="flex items-center gap-2">
							<div
								className="w-3 h-3 rounded-sm"
								style={{ backgroundColor: color }}
							/>
							<span className="font-medium">{value || 0} applications</span>
						</div>
						<div className="text-xs text-muted-foreground mt-1">
							{new Date(day).toLocaleDateString("en-US", {
								weekday: "long",
								year: "numeric",
								month: "long",
								day: "numeric",
							})}
						</div>
					</div>
				)}
				legends={[
					{
						anchor: "bottom-right",
						direction: "row",
						translateY: 36,
						itemCount: 4,
						itemWidth: 42,
						itemHeight: 36,
						itemsSpacing: 14,
						itemDirection: "right-to-left",
						itemTextColor: "hsl(var(--muted-foreground))",
					},
				]}
			/>
		</Card>
	);
}

import { Briefcase, Star, Search, CheckCircle, XCircle, Clock } from "lucide-react";
import { format, isToday, isYesterday, isThisWeek } from "date-fns";
import { Link } from "@tanstack/react-router";

const statusConfig = {
  wishlist: { label: "Wishlist", color: "text-yellow-600", bg: "bg-yellow-50", borderColor: "border-yellow-200", icon: Star },
  applied: { label: "Applied", color: "text-blue-600", bg: "bg-blue-50", borderColor: "border-blue-200", icon: Search },
  interviewing: { label: "Interviewing", color: "text-purple-600", bg: "bg-purple-50", borderColor: "border-purple-200", icon: Clock },
  offered: { label: "Offered", color: "text-green-600", bg: "bg-green-50", borderColor: "border-green-200", icon: CheckCircle },
  rejected: { label: "Rejected", color: "text-red-600", bg: "bg-red-50", borderColor: "border-red-200", icon: XCircle },
  withdrawn: { label: "Withdrawn", color: "text-gray-600", bg: "bg-gray-50", borderColor: "border-gray-200", icon: XCircle },
};

type ApplicationEvent = {
  id: string;
  applicationId: string;
  previousStatus: string | null;
  newStatus: string;
  createdAt: Date | string;
  companyName: string;
  jobTitle: string;
};

function getDateGroup(date: Date | string): string {
  const d = typeof date === "string" ? new Date(date) : date;
  if (isToday(d)) return "Today";
  if (isYesterday(d)) return "Yesterday";
  if (isThisWeek(d)) return "This Week";
  return "Earlier";
}

function StatusChangeIcon({ status }: { status: string }) {
  const config = statusConfig[status as keyof typeof statusConfig];
  if (!config) return <Briefcase className="w-4 h-4" />;
  const Icon = config.icon;
  return <Icon className="w-4 h-4" />;
}

function TimelineItem({ event }: { event: ApplicationEvent }) {
  const newConfig = statusConfig[event.newStatus as keyof typeof statusConfig] || statusConfig.applied;
  const prevConfig = event.previousStatus ? statusConfig[event.previousStatus as keyof typeof statusConfig] : null;

  return (
    <Link
      to="/applications/$applicationId"
      params={{ applicationId: event.applicationId }}
      className="flex gap-4 group"
    >
      <div className="flex flex-col items-center">
        <div className={`w-8 h-8 rounded-full flex items-center justify-center ${newConfig.bg} ${newConfig.color}`}>
          <StatusChangeIcon status={event.newStatus} />
        </div>
        <div className="w-px h-full bg-border mt-2" />
      </div>
      <div className="pb-8">
        <div className="flex items-center gap-2 mb-1">
          {prevConfig ? (
            <span className="text-sm text-muted-foreground">
              {prevConfig.label} → <span className={newConfig.color}>{newConfig.label}</span>
            </span>
          ) : (
            <span className={`text-sm ${newConfig.color} font-medium`}>
              Added to {newConfig.label}
            </span>
          )}
        </div>
        <p className="font-medium group-hover:text-foreground text-foreground transition-colors">
          {event.jobTitle} at {event.companyName}
        </p>
        <p className="text-sm text-muted-foreground">
          {format(typeof event.createdAt === "string" ? new Date(event.createdAt) : event.createdAt, "MMM d, yyyy 'at' h:mm a")}
        </p>
      </div>
    </Link>
  );
}

export function Timeline({ events }: { events: ApplicationEvent[] }) {
  if (events.length === 0) {
    return (
      <div className="text-center py-12">
        <Briefcase className="w-12 h-12 text-muted-foreground mx-auto mb-4" />
        <h3 className="text-lg font-medium mb-2">No events yet</h3>
        <p className="text-muted-foreground">
          Your job search timeline will appear here as you track applications.
        </p>
      </div>
    );
  }

  const grouped = events.reduce<Record<string, ApplicationEvent[]>>((acc, event) => {
    const group = getDateGroup(event.createdAt);
    if (!acc[group]) acc[group] = [];
    acc[group].push(event);
    return acc;
  }, {});

  const groupOrder = ["Today", "Yesterday", "This Week", "Earlier"];

  return (
    <div className="space-y-8">
      {groupOrder.map((group) => {
        const groupEvents = grouped[group];
        if (!groupEvents || groupEvents.length === 0) return null;

        return (
          <div key={group}>
            <h3 className="text-sm font-medium text-muted-foreground mb-4">{group}</h3>
            <div className="space-y-0">
              {groupEvents.map((event) => (
                <TimelineItem key={event.id} event={event} />
              ))}
            </div>
          </div>
        );
      })}
    </div>
  );
}

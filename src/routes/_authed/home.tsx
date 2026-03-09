import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useApplicationStats } from "#/lib/applications/hooks";
import { Briefcase, Star, Search, CheckCircle, XCircle, Clock } from "lucide-react";
import { format } from "date-fns";

export const Route = createFileRoute("/_authed/home")({
  component: RouteComponent,
});

const statusConfig = {
  wishlist: { label: "Wishlist", color: "text-yellow-600", bg: "bg-yellow-50", icon: Star },
  applied: { label: "Applied", color: "text-blue-600", bg: "bg-blue-50", icon: Search },
  interviewing: { label: "Interviewing", color: "text-purple-600", bg: "bg-purple-50", icon: Clock },
  offered: { label: "Offered", color: "text-green-600", bg: "bg-green-50", icon: CheckCircle },
  rejected: { label: "Rejected", color: "text-red-600", bg: "bg-red-50", icon: XCircle },
  withdrawn: { label: "Withdrawn", color: "text-gray-600", bg: "bg-gray-50", icon: XCircle },
};

function StatCard({ title, value, icon: Icon, color }: { title: string; value: number; icon: React.ElementType; color: string }) {
  return (
    <div className="bg-card border rounded-lg p-4">
      <div className="flex items-center gap-3">
        <div className={`p-2 rounded-lg ${color}`}>
          <Icon className="w-5 h-5" />
        </div>
        <div>
          <p className="text-sm text-muted-foreground">{title}</p>
          <p className="text-2xl font-bold">{value}</p>
        </div>
      </div>
    </div>
  );
}

function StatusBadge({ status }: { status: string }) {
  const config = statusConfig[status as keyof typeof statusConfig] || statusConfig.applied;
  const Icon = config.icon;
  return (
    <span className={`inline-flex items-center gap-1 px-2 py-1 rounded-full text-xs font-medium ${config.bg} ${config.color}`}>
      <Icon className="w-3 h-3" />
      {config.label}
    </span>
  );
}

function RouteComponent() {
  const { session } = Route.useRouteContext();
  const navigate = useNavigate();
  const { data: stats, isLoading } = useApplicationStats();

  const handleRowClick = (applicationId: string) => {
    navigate({ to: "/applications/$applicationId", params: { applicationId } });
  };

  return (
    <div className="p-8">
      <div className="mb-8">
        <h1 className="text-2xl font-bold">Welcome, {session.user.name}</h1>
        <p className="text-muted-foreground">{session.user.email}</p>
      </div>

      {isLoading ? (
        <div className="text-center py-8 text-muted-foreground">Loading stats...</div>
      ) : (
        <>
          <div className="mb-8">
            <h2 className="text-lg font-semibold mb-4">Overview</h2>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
              <StatCard
                title="Total"
                value={stats?.total ?? 0}
                icon={Briefcase}
                color="bg-slate-100 text-slate-600"
              />
              <StatCard
                title="Wishlist"
                value={stats?.byStatus.wishlist ?? 0}
                icon={Star}
                color="bg-yellow-100 text-yellow-600"
              />
              <StatCard
                title="Applied"
                value={stats?.byStatus.applied ?? 0}
                icon={Search}
                color="bg-blue-100 text-blue-600"
              />
              <StatCard
                title="Interviewing"
                value={stats?.byStatus.interviewing ?? 0}
                icon={Clock}
                color="bg-purple-100 text-purple-600"
              />
              <StatCard
                title="Offered"
                value={stats?.byStatus.offered ?? 0}
                icon={CheckCircle}
                color="bg-green-100 text-green-600"
              />
              <StatCard
                title="Rejected"
                value={stats?.byStatus.rejected ?? 0}
                icon={XCircle}
                color="bg-red-100 text-red-600"
              />
              <StatCard
                title="Withdrawn"
                value={stats?.byStatus.withdrawn ?? 0}
                icon={XCircle}
                color="bg-gray-100 text-gray-600"
              />
            </div>
          </div>

          <div>
            <h2 className="text-lg font-semibold mb-4">Recent Applications</h2>
            {stats?.recent && stats.recent.length > 0 ? (
              <div className="bg-card border rounded-lg overflow-hidden">
                <table className="w-full">
                  <thead className="bg-muted/50">
                    <tr>
                      <th className="text-left p-3 text-sm font-medium text-muted-foreground">Company</th>
                      <th className="text-left p-3 text-sm font-medium text-muted-foreground">Title</th>
                      <th className="text-left p-3 text-sm font-medium text-muted-foreground">Status</th>
                      <th className="text-left p-3 text-sm font-medium text-muted-foreground">Date</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y">
                    {stats.recent.map((app) => (
                      <tr
                        key={app.id}
                        onClick={() => handleRowClick(app.id)}
                        className="hover:bg-muted/30 cursor-pointer transition-colors"
                      >
                        <td className="p-3 font-medium">{app.companyName}</td>
                        <td className="p-3">{app.jobTitle}</td>
                        <td className="p-3">
                          <StatusBadge status={app.status} />
                        </td>
                        <td className="p-3 text-muted-foreground text-sm">
                          {app.applicationDate ? format(new Date(app.applicationDate), "MMM d, yyyy") : "—"}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <div className="text-center py-8 text-muted-foreground bg-card border rounded-lg">
                <Briefcase className="w-12 h-12 mx-auto mb-3 opacity-50" />
                <p>No applications yet</p>
                <p className="text-sm">Add your first application in the Applications page</p>
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
}

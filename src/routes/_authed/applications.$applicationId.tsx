import { createFileRoute, Link } from "@tanstack/react-router";
import { useApplication } from "#/lib/applications/hooks";

export const Route = createFileRoute("/_authed/applications/$applicationId")({
  component: RouteComponent,
});

function RouteComponent() {
  const { applicationId } = Route.useParams();
  const { data: application, isLoading } = useApplication(applicationId);

  if (isLoading) {
    return <div className="p-8">Loading...</div>;
  }

  if (!application) {
    return (
      <div className="p-8 space-y-4">
        <h1 className="text-2xl font-semibold">Application not found</h1>
        <Link to="/applications" className="text-sm underline">
          Back to applications
        </Link>
      </div>
    );
  }

  return (
    <div className="p-8 max-w-3xl">
      <div className="mb-8">
        <Link to="/applications" className="text-sm underline">
          Back to applications
        </Link>
        <h1 className="text-3xl font-bold mt-3">{application.jobTitle}</h1>
        <p className="text-muted-foreground">{application.companyName}</p>
      </div>

      <div className="space-y-4">
        <DetailRow label="Status" value={formatStatus(application.status)} />
        <DetailRow label="Applied On" value={formatDate(application.applicationDate)} />
        {application.platform ? <DetailRow label="Platform" value={application.platform} /> : null}
        {application.location ? <DetailRow label="Location" value={application.location} /> : null}
        {application.salaryRange ? (
          <DetailRow label="Salary Range" value={application.salaryRange} />
        ) : null}
        {application.resumeVersion ? (
          <DetailRow label="Resume Version" value={application.resumeVersion} />
        ) : null}
        {application.jobUrl ? (
          <div className="grid gap-1">
            <p className="text-sm text-muted-foreground">Job URL</p>
            <a
              href={application.jobUrl}
              target="_blank"
              rel="noreferrer"
              className="underline break-all"
            >
              {application.jobUrl}
            </a>
          </div>
        ) : null}
        {application.notes ? <DetailRow label="Notes" value={application.notes} /> : null}
      </div>
    </div>
  );
}

function DetailRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid gap-1">
      <p className="text-sm text-muted-foreground">{label}</p>
      <p className="whitespace-pre-wrap">{value}</p>
    </div>
  );
}

function formatStatus(status: string) {
  return status.charAt(0).toUpperCase() + status.slice(1);
}

function formatDate(value: Date | string) {
  const date = value instanceof Date ? value : new Date(value);
  return new Intl.DateTimeFormat("en-US", { dateStyle: "medium" }).format(date);
}

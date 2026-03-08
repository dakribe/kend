import { createFileRoute, Link } from "@tanstack/react-router";
import { useApplication, useDeleteApplication } from "#/lib/applications/hooks";
import { ArrowLeftIcon, CalendarIcon, MapPinIcon, BanknoteIcon, FileTextIcon, ExternalLinkIcon, StickyNoteIcon, Trash2Icon } from "lucide-react";
import { useState } from "react";
import { Button } from "#/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "#/components/ui/dialog";

export const Route = createFileRoute("/_authed/applications/$applicationId")({
  component: RouteComponent,
});

const statusConfig: Record<string, { label: string; bg: string; text: string; border: string }> = {
  wishlist: { label: "Wishlist", bg: "bg-muted", text: "text-muted-foreground", border: "border-border" },
  applied: { label: "Applied", bg: "bg-blue-50 dark:bg-blue-950", text: "text-blue-700 dark:text-blue-300", border: "border-blue-200 dark:border-blue-800" },
  interviewing: { label: "Interviewing", bg: "bg-amber-50 dark:bg-amber-950", text: "text-amber-700 dark:text-amber-300", border: "border-amber-200 dark:border-amber-800" },
  offered: { label: "Offered", bg: "bg-emerald-50 dark:bg-emerald-950", text: "text-emerald-700 dark:text-emerald-300", border: "border-emerald-200 dark:border-emerald-800" },
  rejected: { label: "Rejected", bg: "bg-rose-50 dark:bg-rose-950", text: "text-rose-700 dark:text-rose-300", border: "border-rose-200 dark:border-rose-800" },
  withdrawn: { label: "Withdrawn", bg: "bg-muted", text: "text-muted-foreground", border: "border-border" },
};

function RouteComponent() {
  const { applicationId } = Route.useParams();
  const { data: application, isLoading } = useApplication(applicationId);
  const deleteMutation = useDeleteApplication();
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);

  if (isLoading) {
    return (
      <div className="min-h-[60vh] flex items-center justify-center">
        <div className="flex flex-col items-center gap-3">
          <div className="w-6 h-6 border-2 border-primary border-t-transparent rounded-full animate-spin" />
          <p className="text-sm text-muted-foreground">Loading application...</p>
        </div>
      </div>
    );
  }

  if (!application) {
    return (
      <div className="min-h-[60vh] flex items-center justify-center">
        <div className="text-center space-y-4">
          <div className="w-16 h-16 mx-auto rounded-full bg-muted flex items-center justify-center">
            <FileTextIcon className="w-8 h-8 text-muted-foreground" />
          </div>
          <h1 className="text-2xl font-semibold">Application not found</h1>
          <p className="text-muted-foreground">This application may have been deleted or you don't have access.</p>
          <Link
            to="/applications"
            className="inline-flex items-center gap-2 text-sm text-primary hover:underline"
          >
            <ArrowLeftIcon className="w-4 h-4" />
            Back to applications
          </Link>
        </div>
      </div>
    );
  }

  const status = statusConfig[application.status] ?? statusConfig.wishlist;

  return (
    <div className="min-h-screen bg-background">
      <div className="max-w-4xl mx-auto px-6 py-8 md:py-12">
        <Link
          to="/applications"
          className="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors mb-8 group"
        >
          <ArrowLeftIcon className="w-4 h-4 transition-transform group-hover:-translate-x-0.5" />
          All applications
        </Link>

        <header className="mb-10">
          <div className="flex items-start justify-between gap-4 mb-4">
            <h1 className="text-3xl md:text-4xl font-bold tracking-tight text-foreground leading-tight">
              {application.jobTitle}
            </h1>
            <span
              className={`shrink-0 px-3 py-1.5 rounded-full text-xs font-medium border ${status.bg} ${status.text} ${status.border}`}
            >
              {status.label}
            </span>
          </div>
          <p className="text-lg text-muted-foreground font-light">
            {application.companyName}
            {application.location && (
              <>
                <span className="mx-2 text-border">·</span>
                <span className="inline-flex items-center gap-1">
                  <MapPinIcon className="w-4 h-4" />
                  {application.location}
                </span>
              </>
            )}
          </p>
        </header>

        <div className="grid md:grid-cols-2 gap-6">
          <SectionCard title="Application" icon={CalendarIcon}>
            <DetailItem label="Applied on" value={formatDate(application.applicationDate)} />
            {application.platform && <DetailItem label="Platform" value={application.platform} />}
            {application.resumeVersion && <DetailItem label="Resume" value={application.resumeVersion} />}
          </SectionCard>

          <SectionCard title="Compensation" icon={BanknoteIcon}>
            {application.salaryRange ? (
              <DetailItem label="Salary range" value={application.salaryRange} />
            ) : (
              <p className="text-sm text-muted-foreground italic">Not specified</p>
            )}
          </SectionCard>
        </div>

        {application.jobUrl && (
          <SectionCard title="Job Posting" icon={ExternalLinkIcon} className="mt-6">
            <a
              href={application.jobUrl}
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-2 text-primary hover:underline break-all font-mono text-sm"
            >
              {new URL(application.jobUrl).hostname}
              <ExternalLinkIcon className="w-3 h-3" />
            </a>
          </SectionCard>
        )}

        {application.notes && (
          <SectionCard title="Notes" icon={StickyNoteIcon} className="mt-6">
            <p className="text-sm leading-relaxed whitespace-pre-wrap text-foreground/90">
              {application.notes}
            </p>
          </SectionCard>
        )}

        <footer className="mt-12 pt-6 border-t flex items-center justify-between">
          <p className="text-xs text-muted-foreground">
            Application ID: <span className="font-mono">{application.id}</span>
          </p>
          <Dialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
            <DialogTrigger asChild>
              <Button variant="ghost" size="sm" className="text-muted-foreground hover:text-destructive">
                <Trash2Icon className="w-4 h-4 mr-2" />
                Delete
              </Button>
            </DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>Delete application?</DialogTitle>
                <DialogDescription>
                  This will permanently remove your application for {application.jobTitle} at {application.companyName}. This action cannot be undone.
                </DialogDescription>
              </DialogHeader>
              <DialogFooter>
                <Button variant="outline" onClick={() => setDeleteDialogOpen(false)}>
                  Cancel
                </Button>
                <Button
                  variant="destructive"
                  onClick={() => deleteMutation.mutate(applicationId)}
                  disabled={deleteMutation.isPending}
                >
                  {deleteMutation.isPending ? "Deleting..." : "Delete"}
                </Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>
        </footer>
      </div>
    </div>
  );
}

function SectionCard({
  title,
  icon: Icon,
  children,
  className = "",
}: {
  title: string;
  icon: React.ElementType;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div className={`bg-card border rounded-xl p-5 ${className}`}>
      <div className="flex items-center gap-2 mb-4">
        <Icon className="w-4 h-4 text-muted-foreground" />
        <h2 className="text-sm font-medium text-muted-foreground uppercase tracking-wider">
          {title}
        </h2>
      </div>
      <div className="space-y-3">{children}</div>
    </div>
  );
}

function DetailItem({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid gap-0.5">
      <p className="text-xs text-muted-foreground">{label}</p>
      <p className="text-sm font-medium text-foreground">{value}</p>
    </div>
  );
}

function formatDate(value: Date | string) {
  const date = value instanceof Date ? value : new Date(value);
  return new Intl.DateTimeFormat("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
  }).format(date);
}

import { Button } from "@/components/ui/button";
import { authClient } from "@/lib/auth/auth-client";
import { createFileRoute } from "@tanstack/react-router";
import { Card } from "@/components/ui/card";
import { 
	BarChart3, 
	Calendar, 
	CheckCircle, 
	Target, 
	TrendingUp, 
	Users,
	ArrowRight,
	Star
} from "lucide-react";

export const Route = createFileRoute("/")({
	component: Home,
	head: () => ({
		meta: [
			{
				title: "Kend - Track Your Job Applications",
			},
		],
	}),
});

function Home() {
	return (
		<div className="min-h-screen bg-gradient-to-br from-background via-background to-muted/20">
			{/* Navigation */}
			<nav className="border-b bg-background/80 backdrop-blur-sm sticky top-0 z-50">
				<div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
					<div className="flex justify-between items-center py-4">
						<div className="flex items-center space-x-2">
							<Target className="h-8 w-8 text-primary" />
							<span className="text-2xl font-bold">Kend</span>
						</div>
						<Button
							onClick={() =>
								authClient.signIn.social({
									provider: "google",
									callbackURL: "/dashboard",
								})
							}
							className="gap-2"
						>
							Get Started
							<ArrowRight className="h-4 w-4" />
						</Button>
					</div>
				</div>
			</nav>

			{/* Hero Section */}
			<section className="px-4 sm:px-6 lg:px-8 py-20">
				<div className="max-w-7xl mx-auto text-center">
					<div className="max-w-3xl mx-auto">
						<h1 className="text-5xl sm:text-6xl font-bold tracking-tight mb-6">
							Track Your Job Hunt
							<span className="text-primary block">Like a Pro</span>
						</h1>
						<p className="text-xl text-muted-foreground mb-8 leading-relaxed">
							Never lose track of applications again. Organize, monitor, and optimize your job search with powerful analytics and insights.
						</p>
						<div className="flex justify-center">
							<Button
								size="lg"
								onClick={() =>
									authClient.signIn.social({
										provider: "google",
										callbackURL: "/dashboard",
									})
								}
								className="gap-2 text-lg px-8 py-6"
							>
								Start Tracking Free
								<ArrowRight className="h-5 w-5" />
							</Button>
						</div>
					</div>
				</div>
			</section>

			{/* Features Section */}
			<section className="px-4 sm:px-6 lg:px-8 py-20 bg-muted/30">
				<div className="max-w-7xl mx-auto">
					<div className="text-center mb-16">
						<h2 className="text-3xl sm:text-4xl font-bold mb-4">
							Everything You Need to Land Your Dream Job
						</h2>
						<p className="text-xl text-muted-foreground max-w-2xl mx-auto">
							Powerful features designed to keep you organized and ahead of the competition.
						</p>
					</div>

					<div className="grid md:grid-cols-2 lg:grid-cols-3 gap-8">
						<Card className="p-6 hover:shadow-lg transition-shadow">
							<div className="flex items-center gap-3 mb-4">
								<div className="bg-primary/10 p-2 rounded-lg">
									<BarChart3 className="h-6 w-6 text-primary" />
								</div>
								<h3 className="text-xl font-semibold">Application Tracking</h3>
							</div>
							<p className="text-muted-foreground">
								Keep track of every application with status updates, company details, and application dates.
							</p>
						</Card>

						<Card className="p-6 hover:shadow-lg transition-shadow">
							<div className="flex items-center gap-3 mb-4">
								<div className="bg-primary/10 p-2 rounded-lg">
									<Calendar className="h-6 w-6 text-primary" />
								</div>
								<h3 className="text-xl font-semibold">Visual Calendar</h3>
							</div>
							<p className="text-muted-foreground">
								See your application timeline at a glance with our beautiful calendar visualization.
							</p>
						</Card>

						<Card className="p-6 hover:shadow-lg transition-shadow">
							<div className="flex items-center gap-3 mb-4">
								<div className="bg-primary/10 p-2 rounded-lg">
									<TrendingUp className="h-6 w-6 text-primary" />
								</div>
								<h3 className="text-xl font-semibold">Analytics & Insights</h3>
							</div>
							<p className="text-muted-foreground">
								Get insights into your job search patterns and optimize your application strategy.
							</p>
						</Card>

						<Card className="p-6 hover:shadow-lg transition-shadow">
							<div className="flex items-center gap-3 mb-4">
								<div className="bg-primary/10 p-2 rounded-lg">
									<CheckCircle className="h-6 w-6 text-primary" />
								</div>
								<h3 className="text-xl font-semibold">Status Management</h3>
							</div>
							<p className="text-muted-foreground">
								Easily update application statuses and track your progress through the hiring process.
							</p>
						</Card>

						<Card className="p-6 hover:shadow-lg transition-shadow">
							<div className="flex items-center gap-3 mb-4">
								<div className="bg-primary/10 p-2 rounded-lg">
									<Star className="h-6 w-6 text-primary" />
								</div>
								<h3 className="text-xl font-semibold">Bookmarking</h3>
							</div>
							<p className="text-muted-foreground">
								Bookmark important applications and opportunities to follow up on later.
							</p>
						</Card>

						<Card className="p-6 hover:shadow-lg transition-shadow">
							<div className="flex items-center gap-3 mb-4">
								<div className="bg-primary/10 p-2 rounded-lg">
									<Users className="h-6 w-6 text-primary" />
								</div>
								<h3 className="text-xl font-semibold">Company Insights</h3>
							</div>
							<p className="text-muted-foreground">
								Store company information and track multiple applications to the same employer.
							</p>
						</Card>
					</div>
				</div>
			</section>

			{/* CTA Section */}
			<section className="px-4 sm:px-6 lg:px-8 py-20">
				<div className="max-w-4xl mx-auto text-center">
					<h2 className="text-3xl sm:text-4xl font-bold mb-6">
						Ready to Take Control of Your Job Search?
					</h2>
					<p className="text-xl text-muted-foreground mb-8">
						Join thousands of job seekers who have organized their applications with Kend.
					</p>
					<Button
						size="lg"
						onClick={() =>
							authClient.signIn.social({
								provider: "google",
								callbackURL: "/dashboard",
							})
						}
						className="gap-2 text-lg px-8 py-6"
					>
						Get Started for Free
						<ArrowRight className="h-5 w-5" />
					</Button>
					<p className="text-sm text-muted-foreground mt-4">
						No credit card required • Sign up with Google in seconds
					</p>
				</div>
			</section>

			{/* Footer */}
			<footer className="border-t bg-muted/30 px-4 sm:px-6 lg:px-8 py-12">
				<div className="max-w-7xl mx-auto">
					<div className="flex justify-center items-center">
						<div className="flex items-center space-x-2">
							<Target className="h-6 w-6 text-primary" />
							<span className="text-xl font-bold">Kend</span>
						</div>
					</div>
				</div>
			</footer>
		</div>
	);
}

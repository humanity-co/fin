import { ArrowDownRight, ArrowUpRight, Clock3 } from "lucide-react";
import { Link } from "react-router-dom";
import { MoneyDisplay } from "../../components/data/MoneyDisplay";
import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui/card";
import { Skeleton } from "../../components/ui/skeleton";

export function DashboardHeader({ title, subtitle }: { title: string; subtitle: string }) {
  return <header><p className="text-xs font-semibold uppercase tracking-widest text-primary">SutraERP / FY 2026–27</p><h1 className="mt-1 text-2xl font-bold">{title}</h1><p className="mt-1 text-sm text-muted-foreground">{subtitle}</p></header>;
}

export function KpiCard({ title, amount, value, trend, positive = true, loading = false, href }: { title: string; amount?: number; value?: string; trend?: string; positive?: boolean; loading?: boolean; href?: string }) {
  const content = <Card className="p-5"><div className="flex items-start justify-between"><p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">{title}</p>{trend && <span className={`flex items-center text-xs font-semibold ${positive ? "text-emerald-600" : "text-rose-600"}`}>{positive ? <ArrowUpRight className="mr-1 h-4 w-4" /> : <ArrowDownRight className="mr-1 h-4 w-4" />}{trend}</span>}</div>{loading ? <Skeleton className="mt-3 h-8 w-32" /> : <div className="mt-3 text-2xl font-bold tabular-nums">{amount !== undefined ? <MoneyDisplay amount={amount} variant="compact" /> : value}</div>}{trend && <p className="mt-2 text-xs text-muted-foreground">vs previous period</p>}</Card>;
  return href ? <Link to={href} className="block">{content}</Link> : content;
}

export function BudgetGauge({ label, value, detail, color = "#6366f1" }: { label: string; value: number; detail?: string; color?: string }) {
  const safe = Math.min(100, Math.max(0, value));
  return <Card className="p-5"><div className="flex items-center gap-5"><div className="relative h-24 w-24 shrink-0 rounded-full" style={{ background: `conic-gradient(${color} ${safe * 3.6}deg, #e2e8f0 0deg)` }}><div className="absolute inset-2 flex items-center justify-center rounded-full bg-white text-lg font-bold">{safe}%</div></div><div><p className="font-semibold">{label}</p>{detail && <p className="mt-1 text-sm text-muted-foreground">{detail}</p>}</div></div></Card>;
}

export function ApprovalQueue({ items }: { items: { label: string; amount?: number; age: string; type?: string }[] }) {
  return <Card><CardHeader className="p-5 pb-3"><CardTitle className="text-base">Pending approvals</CardTitle></CardHeader><CardContent className="p-0">{items.length === 0 ? <p className="px-5 pb-5 text-sm text-muted-foreground">No pending approvals</p> : items.map((item, i) => <div key={i} className="flex items-center justify-between border-t px-5 py-3"><div><p className="font-medium">{item.label}</p><p className="text-xs text-muted-foreground">{item.type ?? "Workflow"} · {item.age}</p></div>{item.amount !== undefined && <MoneyDisplay amount={item.amount} variant="compact" />}</div>)}</CardContent></Card>;
}

export function ComplianceTimeline({ items }: { items: { name: string; date: string; status: "Due soon" | "Filed" | "Overdue" }[] }) {
  return <Card><CardHeader className="p-5 pb-3"><CardTitle className="text-base">Compliance calendar</CardTitle></CardHeader><CardContent className="p-0">{items.map((item, i) => <div key={i} className="flex gap-3 border-t px-5 py-3"><div className="mt-1 h-2 w-2 rounded-full bg-primary" /><div className="flex flex-1 items-center justify-between"><div><p className="font-medium">{item.name}</p><p className="text-xs text-muted-foreground">{item.date}</p></div><span className={`rounded-full px-2 py-1 text-[10px] font-bold ${item.status === "Filed" ? "bg-emerald-100 text-emerald-700" : item.status === "Overdue" ? "bg-rose-100 text-rose-700" : "bg-amber-100 text-amber-700"}`}>{item.status}</span></div></div>)}</CardContent></Card>;
}

export function Section({ title, children }: { title: string; children: React.ReactNode }) { return <Card><CardHeader className="p-5 pb-3"><CardTitle className="text-base">{title}</CardTitle></CardHeader><CardContent className="p-5 pt-0">{children}</CardContent></Card>; }
export function LoadingGrid() { return <div className="grid gap-4 md:grid-cols-4">{[1,2,3,4].map(i => <Skeleton key={i} className="h-32 rounded-xl" />)}</div>; }
export const formatDate = (date: string) => new Date(date).toLocaleDateString("en-IN", { day: "2-digit", month: "short", year: "numeric" });

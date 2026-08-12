import { useMemo, useState } from "react";
import { CalendarDays, CheckCircle2, Clock3 } from "lucide-react";
import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button } from "../../components/ui";
import { StatusBadge } from "../../components/data";
import { useMockQuery } from "../../hooks/useMockQuery";
import { MOCK_COMPLIANCE, type ComplianceItem } from "../../lib/mock-data";
import { formatIndianDate } from "../../lib/formatters";

const CATEGORY_COLORS: Record<string, string> = {
  GST: "bg-violet-100 text-violet-700",
  TDS: "bg-sky-100 text-sky-700",
  "Income Tax": "bg-amber-100 text-amber-700",
  Statutory: "bg-emerald-100 text-emerald-700",
  AISHE: "bg-rose-100 text-rose-700",
  NAAC: "bg-indigo-100 text-indigo-700",
  FRC: "bg-teal-100 text-teal-700",
};

export default function ComplianceCalendar() {
  const { data: items, isLoading } = useMockQuery<ComplianceItem[]>(MOCK_COMPLIANCE);
  const [month, setMonth] = useState("July 2026");
  const [category, setCategory] = useState("all");

  const filtered = useMemo(() => {
    if (!items) return [];
    return items.filter((c) => category === "all" || c.category === category);
  }, [items, category]);

  const sorted = useMemo(
    () => [...filtered].sort((a, b) => a.dueDate.localeCompare(b.dueDate)),
    [filtered]
  );

  const counts = useMemo(() => {
    const list = items ?? [];
    return {
      overdue: list.filter((c) => c.status === "Overdue").length,
      dueSoon: list.filter((c) => c.status === "Due Soon").length,
      filed: list.filter((c) => c.status === "Filed").length,
    };
  }, [items]);

  return (
    <div className="animate-in fade-in duration-500 space-y-6">
      <PageHeader
        title="Compliance Calendar"
        description="Statutory, tax and regulatory deadlines across the fiscal year"
        breadcrumbs={[{ label: "Dashboard" }, { label: "Compliance Calendar" }]}
        actions={
          <div className="flex gap-2">
            <select value={month} onChange={(e) => setMonth(e.target.value)} className="h-10 rounded-lg border border-white/60 bg-white/60 px-3 text-xs font-semibold text-slate-700 focus:outline-none">
              {["July 2026", "August 2026", "September 2026", "Q4 2026", "FY 2026-27"].map((m) => <option key={m}>{m}</option>)}
            </select>
            <Button variant="outline" size="sm" className="glass-input hover-lift">Export</Button>
          </div>
        }
      />

      <div className="flex flex-wrap gap-3">
        <Pill label="Overdue" value={counts.overdue} className="bg-rose-50 text-rose-700 border-rose-200" />
        <Pill label="Due This Week" value={counts.dueSoon} className="bg-amber-50 text-amber-700 border-amber-200" />
        <Pill label="Filed" value={counts.filed} className="bg-emerald-50 text-emerald-700 border-emerald-200" />
        <div className="ml-auto">
          <select value={category} onChange={(e) => setCategory(e.target.value)} className="h-9 rounded-lg border border-white/60 bg-white/60 px-3 text-xs font-semibold text-slate-700 focus:outline-none">
            <option value="all">All Categories</option>
            {Object.keys(CATEGORY_COLORS).map((c) => <option key={c}>{c}</option>)}
          </select>
        </div>
      </div>

      <Card className="glass border-white/60 shadow-lg">
        <div className="border-b border-white/40 bg-white/40 p-4 flex items-center gap-2 text-sm font-bold text-slate-800">
          <CalendarDays className="h-4 w-4 text-primary" /> {month} · FY 2026-27 Deadlines
        </div>
        <CardContent className="p-0">
          {isLoading ? (
            <div className="space-y-3 p-6">
              {[1, 2, 3, 4, 5].map((i) => <div key={i} className="h-12 animate-pulse rounded-lg bg-slate-200/50" />)}
            </div>
          ) : sorted.length === 0 ? (
            <div className="flex flex-col items-center justify-center gap-2 py-16 text-center">
              <Clock3 className="h-10 w-10 text-slate-300" />
              <p className="text-sm font-semibold text-slate-600">No compliance items in this category</p>
              <p className="text-xs text-slate-400">Deadlines are driven by the statutory configuration.</p>
            </div>
          ) : (
            <div className="divide-y divide-white/40">
              {sorted.map((c) => (
                <div key={c.id} className="flex items-center gap-4 px-5 py-3.5 hover:bg-white/40 transition-colors">
                  <div className={`w-16 shrink-0 rounded-lg px-2 py-1 text-center text-[9px] font-bold ${CATEGORY_COLORS[c.category] ?? "bg-slate-100 text-slate-600"}`}>
                    {c.category}
                  </div>
                  <div className="flex-1 min-w-0">
                    <p className="truncate text-sm font-semibold text-slate-800">{c.title}</p>
                    <p className="text-[11px] text-slate-400">Responsible: {c.responsible}</p>
                  </div>
                  <div className="text-right">
                    <p className={`font-mono text-xs font-bold ${c.status === "Overdue" ? "text-rose-600" : "text-slate-600"}`}>
                      {formatIndianDate(c.dueDate)}
                    </p>
                    {c.filedOn && (
                      <p className="flex items-center gap-1 text-[10px] font-semibold text-emerald-600">
                        <CheckCircle2 className="h-3 w-3" /> Filed {formatIndianDate(c.filedOn)}
                      </p>
                    )}
                  </div>
                  <div className="w-24 text-right"><StatusBadge status={c.status} /></div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function Pill({ label, value, className }: { label: string; value: number; className: string }) {
  return <div className={`rounded-full border px-4 py-1.5 text-xs font-bold shadow-sm ${className}`}>{label}: {value}</div>;
}

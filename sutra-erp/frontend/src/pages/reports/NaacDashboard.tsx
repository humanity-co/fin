import { useMemo, useState } from "react";
import { Award, Download, Target, TrendingUp } from "lucide-react";
import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button, TableSkeleton } from "../../components/ui";
import { StatusBadge } from "../../components/data";
import { DataTable, type ColumnDef } from "../../components/data/DataTable";
import { useMockQuery } from "../../hooks/useMockQuery";
import { usePagination } from "../../hooks/usePagination";
import { MOCK_NAAC_METRICS, type NaacMetric } from "../../lib/mock-data";

export default function NaacDashboard() {
  const pagination = usePagination(10);
  const { data: metrics, isLoading } = useMockQuery<NaacMetric[]>(MOCK_NAAC_METRICS);
  const [year, setYear] = useState("2025-26");
  const [statusFilter, setStatusFilter] = useState("all");

  const filtered = useMemo(() => {
    if (!metrics) return [];
    return metrics.filter((m) => statusFilter === "all" || m.status === statusFilter);
  }, [metrics, statusFilter]);

  const pageRows = filtered.slice(pagination.offset, pagination.offset + pagination.pageSize);

  const stats = useMemo(() => {
    const list = metrics ?? [];
    return { onTrack: list.filter((m) => m.status === "On Track").length, atRisk: list.filter((m) => m.status === "At Risk").length, done: list.filter((m) => m.status === "Completed").length };
  }, [metrics]);

  const columns: ColumnDef<NaacMetric>[] = [
    { id: "criterion", header: "Criterion", accessorFn: (r) => <span className="font-mono text-xs font-bold text-primary">{r.criterion}</span>, className: "w-24" },
    { id: "key", header: "Metric", accessorFn: (r) => <span className="font-semibold text-slate-800">{r.key}</span> },
    { id: "value", header: "Value", align: "right", accessorFn: (r) => <span className="font-bold text-slate-700">{r.value}</span>, className: "w-24" },
    { id: "target", header: "Target", align: "right", accessorFn: (r) => <span className="text-slate-500">{r.target}</span>, className: "w-24" },
    {
      id: "trend", header: "4-Year Trend",
      accessorFn: (r) => (
        <div className="flex items-end gap-1 h-8">
          {r.trend.map((v, i) => {
            const max = Math.max(...r.trend, 1);
            return <div key={i} className="w-4 rounded-t bg-gradient-to-t from-primary/60 to-primary" style={{ height: `${(v / max) * 100}%` }} title={String(v)} />;
          })}
        </div>
      ),
      className: "w-32",
    },
    { id: "status", header: "Status", accessorFn: (r) => <StatusBadge status={r.status} />, className: "w-28" },
  ];

  return (
    <div className="animate-in fade-in duration-500 space-y-6">
      <PageHeader
        title="NAAC Dashboard"
        description="Metric-wise progress toward NAAC SSR submission"
        breadcrumbs={[{ label: "Reports" }, { label: "NAAC Dashboard" }]}
        actions={
          <div className="flex gap-2">
            <select value={year} onChange={(e) => setYear(e.target.value)} className="h-10 rounded-lg border border-white/60 bg-white/60 px-3 text-xs font-semibold text-slate-700 focus:outline-none">
              <option>2025-26</option><option>2024-25</option><option>2023-24</option>
            </select>
            <Button size="sm" className="bg-primary hover:bg-primary/90 text-white shadow-lg hover-lift"><Download className="h-4 w-4 mr-1" /> Export SSR Data</Button>
          </div>
        }
      />
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <Kpi label="Metrics On Track" value={stats.onTrack} icon={<TrendingUp className="h-4 w-4 text-emerald-600" />} />
        <Kpi label="At Risk" value={stats.atRisk} icon={<Target className="h-4 w-4 text-amber-600" />} />
        <Kpi label="Completed" value={stats.done} icon={<Award className="h-4 w-4 text-violet-600" />} />
        <Kpi label="Overall Readiness" value={`${Math.round(((stats.onTrack + stats.done) / Math.max((metrics ?? []).length, 1)) * 100)}%`} icon={<Target className="h-4 w-4 text-primary" />} />
      </div>
      <Card className="glass border-white/60 shadow-lg">
        <div className="p-4 border-b border-white/40 bg-white/40 backdrop-blur-md flex items-center gap-3">
          <select value={statusFilter} onChange={(e) => setStatusFilter(e.target.value)} className="h-10 rounded-lg border border-white/60 bg-white/60 px-3 text-xs font-semibold text-slate-700 focus:outline-none">
            <option value="all">All Statuses</option>
            <option value="On Track">On Track</option>
            <option value="At Risk">At Risk</option>
            <option value="Completed">Completed</option>
          </select>
          <p className="text-[11px] text-slate-400">Reference year: {year} · SSR draft v1 target: 15-Aug-2026</p>
        </div>
        <CardContent className="p-0 custom-scrollbar overflow-auto">
          {isLoading ? <div className="p-6"><TableSkeleton rows={7} cols={6} /></div>
          : pageRows.length === 0 ? (
            <div className="flex flex-col items-center justify-center gap-2 py-16 text-center">
              <Award className="h-10 w-10 text-slate-300" />
              <p className="text-sm font-semibold text-slate-600">No metrics match the status filter</p>
              <p className="text-xs text-slate-400">Metric data will be computed from finance, scholarship and research modules.</p>
            </div>
          ) : (
            <div className="min-w-[850px]">
              <DataTable data={pageRows} columns={columns} getRowId={(r) => r.key}
                pagination={{ page: pagination.page, pageSize: pagination.pageSize, total: filtered.length, onPageChange: pagination.setPage, onPageSizeChange: pagination.setPageSize }} />
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function Kpi({ label, value, icon }: { label: string; value: React.ReactNode; icon: React.ReactNode }) {
  return (
    <Card className="glass border-white/60 p-4 shadow-sm">
      <div className="flex items-center justify-between">
        <p className="text-[10px] font-bold uppercase tracking-widest text-slate-500">{label}</p>
        {icon}
      </div>
      <p className="mt-1 text-2xl font-extrabold text-slate-800">{value}</p>
    </Card>
  );
}

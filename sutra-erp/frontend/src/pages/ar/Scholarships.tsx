import { useMemo, useState } from "react";
import { Award, CheckCircle2, ExternalLink, GraduationCap, Search } from "lucide-react";
import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button, Input, TableSkeleton, Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "../../components/ui";
import { MoneyDisplay, StatusBadge } from "../../components/data";
import { DataTable, type ColumnDef } from "../../components/data/DataTable";
import { useMockQuery } from "../../hooks/useMockQuery";
import { usePagination } from "../../hooks/usePagination";
import { MOCK_SCHOLARSHIPS, type ScholarshipRecord } from "../../lib/mock-data";
import { formatIndianDate } from "../../lib/formatters";

const STEPS = ["Applied", "Verified", "Sanctioned", "Disbursed", "Reconciled"] as const;

/**
 * Scholarship Management — register of scholarship awards with MahaDBT
 * tracking and a lifecycle stepper per student.
 */
export default function Scholarships() {
  const pagination = usePagination(10);
  const { data: scholarships, isLoading } = useMockQuery<ScholarshipRecord[]>(MOCK_SCHOLARSHIPS);

  const [search, setSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState("all");
  const [selected, setSelected] = useState<ScholarshipRecord | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);

  const filtered = useMemo(() => {
    if (!scholarships) return [];
    return scholarships.filter((s) => {
      if (statusFilter !== "all" && s.status !== statusFilter) return false;
      if (search.trim()) {
        const q = search.toLowerCase();
        return (
          s.student_name.toLowerCase().includes(q) ||
          s.enrollment.toLowerCase().includes(q) ||
          s.scheme.toLowerCase().includes(q) ||
          s.mahadbt_reference.toLowerCase().includes(q)
        );
      }
      return true;
    });
  }, [scholarships, search, statusFilter]);

  const pageRows = useMemo(
    () => filtered.slice(pagination.offset, pagination.offset + pagination.pageSize),
    [filtered, pagination.offset, pagination.pageSize]
  );

  const totals = useMemo(() => {
    const list = scholarships ?? [];
    return {
      sanctioned: list.reduce((s, r) => s + r.sanctioned_amount, 0),
      disbursed: list.reduce((s, r) => s + r.disbursed_amount, 0),
      pending: list.filter((r) => !r.reconciled).length,
    };
  }, [scholarships]);

  const columns: ColumnDef<ScholarshipRecord>[] = [
    {
      id: "student", header: "Student",
      accessorFn: (r) => (
        <button className="text-left cursor-pointer hover:text-primary hover:underline" onClick={() => { setSelected(r); setDialogOpen(true); }}>
          <div className="font-semibold text-slate-800">{r.student_name}</div>
          <div className="font-mono text-[10px] text-slate-400">{r.enrollment}</div>
        </button>
      ),
    },
    { id: "scheme", header: "Scheme", accessorFn: (r) => <div className="max-w-[220px]"><p className="text-xs font-medium text-slate-600 leading-snug">{r.scheme}</p><p className="text-[10px] text-slate-400">{r.agency}</p></div> },
    { id: "sanctioned", header: "Sanctioned (₹)", align: "right", accessorFn: (r) => <MoneyDisplay amount={r.sanctioned_amount} />, className: "w-32" },
    { id: "disbursed", header: "Disbursed (₹)", align: "right", accessorFn: (r) => <MoneyDisplay amount={r.disbursed_amount} />, className: "w-32" },
    { id: "mahadbt", header: "MahaDBT", accessorFn: (r) => (
      <div>
        <StatusBadge status={r.mahadbt_status} />
        <p className="mt-0.5 font-mono text-[10px] text-slate-400">{r.mahadbt_reference}</p>
      </div>
    ) },
    { id: "status", header: "Status", accessorFn: (r) => <StatusBadge status={r.status} />, className: "w-28" },
    { id: "reconciled", header: "Reconciled", accessorFn: (r) => r.reconciled ? <CheckCircle2 className="h-4 w-4 text-emerald-600" /> : <span className="text-[10px] font-bold text-amber-600">Pending</span>, className: "w-24" },
    {
      id: "actions", header: "", align: "right",
      accessorFn: (r) => (
        <Button variant="outline" size="sm" className="h-7 px-2 text-xs" onClick={() => { setSelected(r); setDialogOpen(true); }}>View</Button>
      ),
      className: "w-20",
    },
  ];

  return (
    <div className="animate-in fade-in duration-500 space-y-6">
      <PageHeader
        title="Scholarship Management"
        description="Track sanctioned vs disbursed amounts, MahaDBT status and DBT reconciliation"
        breadcrumbs={[{ label: "Accounts Receivable" }, { label: "Scholarships" }]}
        actions={
          <Button variant="outline" size="sm" className="glass-input hover-lift">
            <ExternalLink className="h-4 w-4 mr-2" /> MahaDBT Portal
          </Button>
        }
      />

      {/* KPI strip */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Kpi label="Total Sanctioned" value={<MoneyDisplay amount={totals.sanctioned} />} tone="text-slate-800" />
        <Kpi label="Total Disbursed" value={<MoneyDisplay amount={totals.disbursed} />} tone="text-emerald-700" />
        <Kpi label="Awaiting Reconciliation" value={String(totals.pending)} tone="text-amber-600" />
      </div>

      <Card className="glass border-white/60 shadow-lg">
        <div className="p-4 border-b border-white/40 bg-white/40 backdrop-blur-md flex flex-wrap items-center gap-3">
          <div className="relative w-80">
            <Search className="absolute left-3 top-2.5 h-4 w-4 text-slate-400" />
            <Input placeholder="Search student, scheme, MahaDBT ref..." className="pl-9 h-10 glass-input shadow-sm" value={search} onChange={(e) => setSearch(e.target.value)} />
          </div>
          <select value={statusFilter} onChange={(e) => setStatusFilter(e.target.value)} className="h-10 rounded-lg border border-white/60 bg-white/60 px-3 text-xs font-semibold text-slate-700 shadow-sm focus:outline-none focus:ring-2 focus:ring-primary/40">
            <option value="all">All Statuses</option>
            {["Applied", "Verified", "Sanctioned", "PartiallyDisbursed", "Disbursed", "Rejected", "Closed"].map((s) => <option key={s} value={s}>{s}</option>)}
          </select>
          <div className="ml-auto text-xs font-bold text-slate-500">{filtered.length} awards</div>
        </div>
        <CardContent className="p-0 custom-scrollbar overflow-auto">
          {isLoading ? (
            <div className="p-6"><TableSkeleton rows={6} cols={7} /></div>
          ) : pageRows.length === 0 ? (
            <div className="flex flex-col items-center justify-center gap-3 py-16 text-center">
              <Award className="h-10 w-10 text-slate-300" />
              <p className="text-sm font-bold text-slate-700">{search || statusFilter !== "all" ? "No scholarships match your filters" : "No scholarships yet"}</p>
              <p className="text-xs text-slate-400 max-w-sm">
                {search || statusFilter !== "all"
                  ? "Try clearing the search or switching the status filter."
                  : "Link students to scholarship schemes to track sanctioning, MahaDBT disbursement and reconciliation."}
              </p>
            </div>
          ) : (
            <div className="min-w-[1050px]">
              <DataTable
                data={pageRows}
                columns={columns}
                getRowId={(r) => r.scholarship_id}
                pagination={{ page: pagination.page, pageSize: pagination.pageSize, total: filtered.length, onPageChange: pagination.setPage, onPageSizeChange: pagination.setPageSize }}
              />
            </div>
          )}
        </CardContent>
      </Card>

      {/* Detail dialog with lifecycle stepper */}
      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-2xl">
          {selected && (
            <>
              <DialogHeader>
                <DialogTitle className="flex items-center gap-2 text-lg"><GraduationCap className="h-5 w-5 text-primary" /> {selected.student_name}</DialogTitle>
                <DialogDescription>
                  {selected.enrollment} · {selected.scheme} · {selected.agency}
                </DialogDescription>
              </DialogHeader>
              <div className="space-y-5 text-sm">
                {/* Lifecycle stepper */}
                <div className="flex items-center justify-between">
                  {STEPS.map((step, i) => {
                    const reached = i < STEPS.indexOf(selected.status === "Disbursed" || selected.status === "PartiallyDisbursed" ? "Disbursed" : selected.status === "Sanctioned" ? "Sanctioned" : selected.status === "Verified" ? "Verified" : selected.status === "Applied" ? "Applied" : "Reconciled");
                    const active = selected.status === "PartiallyDisbursed" && step === "Disbursed";
                    return (
                      <div key={step} className="flex flex-1 flex-col items-center">
                        <div className={`flex h-7 w-7 items-center justify-center rounded-full border-2 text-[10px] font-extrabold ${reached || active ? "border-emerald-500 bg-emerald-500 text-white" : "border-slate-300 bg-white text-slate-400"}`}>
                          {reached ? "✓" : i + 1}
                        </div>
                        <p className={`mt-1 text-[9px] font-bold uppercase tracking-wider ${reached || active ? "text-emerald-700" : "text-slate-400"}`}>{step}</p>
                        {i < STEPS.length - 1 && <div className={`h-0.5 w-full -mt-4 ${reached ? "bg-emerald-400" : "bg-slate-200"}`} />}
                      </div>
                    );
                  })}
                </div>

                <div className="grid grid-cols-2 gap-3">
                  <Info label="Sanctioned Amount" value={<MoneyDisplay amount={selected.sanctioned_amount} />} />
                  <Info label="Disbursed Amount" value={<MoneyDisplay amount={selected.disbursed_amount} />} />
                  <Info label="MahaDBT Reference" value={<span className="font-mono text-[11px]">{selected.mahadbt_reference}</span>} />
                  <Info label="MahaDBT Status" value={<StatusBadge status={selected.mahadbt_status} />} />
                  <Info label="Beneficiary Bank" value={`${selected.bank} (${selected.ifsc})`} />
                  <Info label="Applied On" value={formatIndianDate(selected.applied_on)} />
                </div>

                <div className={`flex items-center gap-2 rounded-lg border px-4 py-3 text-xs font-bold ${selected.reconciled ? "border-emerald-200 bg-emerald-50/70 text-emerald-700" : "border-amber-200 bg-amber-50/70 text-amber-700"}`}>
                  <CheckCircle2 className="h-4 w-4" />
                  {selected.reconciled ? "DBT reconciliation complete — disbursement matches bank records." : "Reconciliation pending — compare disbursed amount with MahaDBT bank statement."}
                </div>
              </div>
              <DialogFooter>
                <Button variant="outline" onClick={() => setDialogOpen(false)}>Close</Button>
                <Button className="bg-primary text-white">Verify / Reconcile</Button>
              </DialogFooter>
            </>
          )}
        </DialogContent>
      </Dialog>
    </div>
  );
}

function Kpi({ label, value, tone }: { label: string; value: React.ReactNode; tone: string }) {
  return (
    <Card className="glass border-white/60 p-4 shadow-sm">
      <p className="text-[10px] font-bold uppercase tracking-widest text-slate-500">{label}</p>
      <p className={`mt-1 text-xl font-extrabold ${tone}`}>{value}</p>
    </Card>
  );
}

function Info({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div className="rounded-lg border border-white/50 bg-white/40 p-3">
      <p className="text-[10px] font-bold uppercase tracking-wider text-slate-500">{label}</p>
      <div className="mt-1 text-sm font-semibold text-slate-700">{value}</div>
    </div>
  );
}

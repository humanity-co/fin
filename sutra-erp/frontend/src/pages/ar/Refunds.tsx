import { useMemo, useState } from "react";
import { Banknote, Calculator, CircleDollarSign, Search, ShieldCheck, Undo2 } from "lucide-react";
import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button, Input, TableSkeleton, Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "../../components/ui";
import { MoneyDisplay, StatusBadge } from "../../components/data";
import { DataTable, type ColumnDef } from "../../components/data/DataTable";
import { useMockQuery } from "../../hooks/useMockQuery";
import { usePagination } from "../../hooks/usePagination";
import { MOCK_REFUNDS, type RefundRequest } from "../../lib/mock-data";
import { formatIndianDate } from "../../lib/formatters";

/**
 * Refund Management — FRC-compliant refund requests with deduction
 * preview, original payment linkage and processing workflow.
 */
export default function Refunds() {
  const pagination = usePagination(10);
  const { data: refunds, isLoading } = useMockQuery<RefundRequest[]>(MOCK_REFUNDS);

  const [search, setSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState("all");
  const [selected, setSelected] = useState<RefundRequest | null>(null);
  const [processing, setProcessing] = useState(false);
  const [dialogOpen, setDialogOpen] = useState(false);

  const filtered = useMemo(() => {
    if (!refunds) return [];
    return refunds.filter((r) => {
      if (statusFilter !== "all" && r.status !== statusFilter) return false;
      if (search.trim()) {
        const q = search.toLowerCase();
        return r.student_name.toLowerCase().includes(q) || r.enrollment.toLowerCase().includes(q) || r.refund_id.toLowerCase().includes(q) || r.original_receipt.toLowerCase().includes(q);
      }
      return true;
    });
  }, [refunds, search, statusFilter]);

  const pageRows = useMemo(
    () => filtered.slice(pagination.offset, pagination.offset + pagination.pageSize),
    [filtered, pagination.offset, pagination.pageSize]
  );

  const totals = useMemo(() => {
    const list = refunds ?? [];
    return {
      pending: list.filter((r) => r.status === "Pending").length,
      approved: list.filter((r) => r.status === "Approved").length,
      totalRefundable: list.filter((r) => r.status !== "Rejected").reduce((s, r) => s + r.refundable_amount, 0),
    };
  }, [refunds]);

  const openProcess = (r: RefundRequest) => { setSelected(r); setProcessing(r.status === "Pending"); setDialogOpen(true); };

  const columns: ColumnDef<RefundRequest>[] = [
    {
      id: "student", header: "Student",
      accessorFn: (r) => (
        <button className="text-left cursor-pointer hover:text-primary hover:underline" onClick={() => openProcess(r)}>
          <div className="font-semibold text-slate-800">{r.student_name}</div>
          <div className="font-mono text-[10px] text-slate-400">{r.enrollment}</div>
        </button>
      ),
    },
    { id: "original", header: "Original Payment", accessorFn: (r) => (
      <div>
        <p className="font-mono text-xs font-bold text-primary">{r.original_receipt}</p>
        <p className="text-[10px] font-mono text-slate-400">{formatIndianDate(r.original_payment_date)}</p>
      </div>
    ) },
    { id: "original_amount", header: "Paid (₹)", align: "right", accessorFn: (r) => <MoneyDisplay amount={r.original_amount} />, className: "w-32" },
    {
      id: "refundable", header: "Refundable (₹)", align: "right", sortable: true,
      accessorFn: (r) => <span className="font-bold text-emerald-700"><MoneyDisplay amount={r.refundable_amount} /></span>,
      className: "w-36",
    },
    { id: "frc", header: "FRC Reference", accessorFn: (r) => (
      <div className="flex items-center gap-1.5">
        <ShieldCheck className="h-3.5 w-3.5 text-emerald-600" />
        <span className="font-mono text-[10px] text-slate-600">{r.frc_reference}</span>
      </div>
    ) },
    { id: "status", header: "Status", accessorFn: (r) => <StatusBadge status={r.status} />, className: "w-24" },
    {
      id: "actions", header: "", align: "right",
      accessorFn: (r) => (
        <div className="flex justify-end gap-1.5">
          <Button variant="outline" size="sm" className="h-7 px-2 text-xs" onClick={() => openProcess(r)}>View</Button>
          {r.status === "Pending" && (
            <Button variant="outline" size="sm" className="h-7 px-2 text-xs text-emerald-700 border-emerald-200 hover:bg-emerald-50" onClick={() => openProcess(r)}>Process</Button>
          )}
        </div>
      ),
      className: "w-36",
    },
  ];

  return (
    <div className="animate-in fade-in duration-500 space-y-6">
      <PageHeader
        title="Refund Management"
        description="FRC-compliant refund requests with deduction preview and processing"
        breadcrumbs={[{ label: "Accounts Receivable" }, { label: "Refunds" }]}
        actions={
          <Button size="sm" className="bg-primary hover:bg-primary/90 text-white shadow-lg hover-lift">
            <Undo2 className="h-4 w-4 mr-1" /> Initiate Refund
          </Button>
        }
      />

      {/* KPI strip */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Kpi label="Pending Requests" value={String(totals.pending)} tone="text-amber-600" />
        <Kpi label="Approved — Awaiting Process" value={String(totals.approved)} tone="text-sky-700" />
        <Kpi label="Total Refundable (Open)" value={<MoneyDisplay amount={totals.totalRefundable} />} tone="text-slate-800" />
      </div>

      <Card className="glass border-white/60 shadow-lg">
        <div className="p-4 border-b border-white/40 bg-white/40 backdrop-blur-md flex flex-wrap items-center gap-3">
          <div className="relative w-80">
            <Search className="absolute left-3 top-2.5 h-4 w-4 text-slate-400" />
            <Input placeholder="Search student, receipt, FRC ref..." className="pl-9 h-10 glass-input shadow-sm" value={search} onChange={(e) => setSearch(e.target.value)} />
          </div>
          <select value={statusFilter} onChange={(e) => setStatusFilter(e.target.value)} className="h-10 rounded-lg border border-white/60 bg-white/60 px-3 text-xs font-semibold text-slate-700 shadow-sm focus:outline-none focus:ring-2 focus:ring-primary/40">
            <option value="all">All Statuses</option>
            <option value="Pending">Pending</option>
            <option value="Approved">Approved</option>
            <option value="Processed">Processed</option>
            <option value="Rejected">Rejected</option>
          </select>
          <div className="ml-auto text-xs font-bold text-slate-500">{filtered.length} requests</div>
        </div>
        <CardContent className="p-0 custom-scrollbar overflow-auto">
          {isLoading ? (
            <div className="p-6"><TableSkeleton rows={5} cols={7} /></div>
          ) : pageRows.length === 0 ? (
            <div className="flex flex-col items-center justify-center gap-3 py-16 text-center">
              <Banknote className="h-10 w-10 text-slate-300" />
              <p className="text-sm font-bold text-slate-700">{search || statusFilter !== "all" ? "No refund requests match your filters" : "No refund requests yet"}</p>
              <p className="text-xs text-slate-400 max-w-sm">
                {search || statusFilter !== "all"
                  ? "Try clearing the search or switching the status filter."
                  : "Initiate a refund to process FRC-compliant fee returns for withdrawn or overpaid students."}
              </p>
            </div>
          ) : (
            <div className="min-w-[1050px]">
              <DataTable
                data={pageRows}
                columns={columns}
                getRowId={(r) => r.refund_id}
                pagination={{ page: pagination.page, pageSize: pagination.pageSize, total: filtered.length, onPageChange: pagination.setPage, onPageSizeChange: pagination.setPageSize }}
              />
            </div>
          )}
        </CardContent>
      </Card>

      {/* Process refund dialog */}
      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-2xl">
          {selected && (
            <ProcessRefundForm refund={selected} isProcessing={processing} onDone={() => setDialogOpen(false)} />
          )}
        </DialogContent>
      </Dialog>
    </div>
  );
}

function ProcessRefundForm({ refund, isProcessing, onDone }: { refund: RefundRequest; isProcessing: boolean; onDone: () => void }) {
  const [mode, setMode] = useState(refund.mode);
  const [confirm, setConfirm] = useState(false);

  const handleProcess = () => {
    alert(
      `Refund ${refund.refund_id.toUpperCase()} processed via ${mode}.\nAmount: ${(refund.refundable_amount / 100).toLocaleString("en-IN", { style: "currency", currency: "INR" })}\nBeneficiary: ${refund.beneficiary}`
    );
    onDone();
  };

  return (
    <>
      <DialogHeader>
        <DialogTitle className="flex items-center gap-2 text-lg">
          <Calculator className="h-5 w-5 text-primary" />
          {isProcessing ? `Process Refund — ${refund.student_name}` : `Refund Detail — ${refund.student_name}`}
        </DialogTitle>
        <DialogDescription>
          {refund.frc_reference} · Original receipt {refund.original_receipt} on {formatIndianDate(refund.original_payment_date)}
        </DialogDescription>
      </DialogHeader>

      <div className="space-y-4 text-sm">
        {/* FRC calculation preview */}
        <div className="rounded-xl border border-white/50 bg-white/40 p-4">
          <p className="mb-2 flex items-center gap-1.5 text-xs font-bold uppercase tracking-wider text-slate-500">
            <ShieldCheck className="h-4 w-4 text-emerald-600" /> FRC-Compliant Calculation Preview
          </p>
          <div className="space-y-1.5">
            <div className="flex justify-between text-xs font-semibold text-slate-600">
              <span>Original amount paid</span><span><MoneyDisplay amount={refund.original_amount} /></span>
            </div>
            {refund.deduction_breakup.map((d) => (
              <div key={d.head} className="flex justify-between text-xs text-slate-500">
                <span className="max-w-[70%]">{d.head}</span>
                <span className="text-rose-600">− <MoneyDisplay amount={d.amount} /></span>
              </div>
            ))}
            <div className="flex justify-between border-t border-dashed border-white/50 pt-2 text-sm font-extrabold text-emerald-700">
              <span>Refundable amount</span><span><MoneyDisplay amount={refund.refundable_amount} /></span>
            </div>
          </div>
        </div>

        <div className="grid grid-cols-2 gap-3">
          <div>
            <p className="mb-1 text-[10px] font-bold uppercase tracking-wider text-slate-500">Reason</p>
            <p className="text-xs text-slate-600">{refund.reason}</p>
          </div>
          <div>
            <p className="mb-1 text-[10px] font-bold uppercase tracking-wider text-slate-500">Beneficiary</p>
            <p className="text-xs font-semibold text-slate-700">{refund.beneficiary}</p>
            <p className="font-mono text-[10px] text-slate-400">{refund.account_no} · {refund.ifsc}</p>
          </div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          <div>
            <label className="mb-1 block text-[10px] font-bold uppercase tracking-wider text-slate-500">Refund Mode</label>
            <select value={mode} onChange={(e) => setMode(e.target.value as typeof mode)} disabled={!isProcessing} className="h-10 w-full rounded-lg border border-white/60 bg-white/60 px-3 text-sm font-semibold text-slate-700 focus:outline-none focus:ring-2 focus:ring-primary/40 disabled:opacity-50">
              <option>NEFT</option><option>RTGS</option><option>UPI</option><option>Cheque</option>
            </select>
          </div>
          <div>
            <label className="mb-1 block text-[10px] font-bold uppercase tracking-wider text-slate-500">Reference (optional)</label>
            <Input placeholder="UTR / cheque no." className="glass-input" />
          </div>
        </div>

        {isProcessing && (
          <label className="flex items-center gap-2 rounded-lg border border-rose-200 bg-rose-50/60 px-4 py-3 text-xs font-bold text-rose-700">
            <input type="checkbox" checked={confirm} onChange={(e) => setConfirm(e.target.checked)} className="h-4 w-4" />
            I confirm the deduction is per FRC clause 7 and the beneficiary details are verified.
          </label>
        )}
      </div>

      <DialogFooter>
        <Button variant="outline" onClick={onDone}>Close</Button>
        {isProcessing ? (
          <Button className="bg-emerald-600 text-white hover:bg-emerald-500" disabled={!confirm} onClick={handleProcess}>
            <CircleDollarSign className="h-4 w-4 mr-1" /> Approve & Process Refund
          </Button>
        ) : (
          refund.status === "Pending" && <Button className="bg-emerald-600 text-white">Approve</Button>
        )}
      </DialogFooter>
    </>
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

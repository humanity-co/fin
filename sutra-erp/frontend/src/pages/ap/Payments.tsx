import { useMemo, useState } from "react";
import { Banknote, CalendarClock, Landmark, Percent, Search, Send, Wallet } from "lucide-react";
import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button, Input, TableSkeleton, Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "../../components/ui";
import { MoneyDisplay, StatusBadge } from "../../components/data";
import { DataTable, type ColumnDef } from "../../components/data/DataTable";
import { useMockQuery } from "../../hooks/useMockQuery";
import { usePagination } from "../../hooks/usePagination";
import { MOCK_PAYMENTS, MOCK_BANK_ACCOUNTS, type PaymentDue } from "../../lib/mock-data";
import { formatIndianDate } from "../../lib/formatters";

/**
 * Payment Processing — due payments register with TDS auto-calculation
 * (section + rate), bank account picker and payment modes.
 */
export default function Payments() {
  const pagination = usePagination(10);
  const { data: payments, isLoading } = useMockQuery<PaymentDue[]>(MOCK_PAYMENTS);

  const [search, setSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState("all");
  const [selected, setSelected] = useState<PaymentDue | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);

  const filtered = useMemo(() => {
    if (!payments) return [];
    return payments.filter((p) => {
      if (statusFilter !== "all" && p.status !== statusFilter) return false;
      if (search.trim()) {
        const q = search.toLowerCase();
        return p.vendorName.toLowerCase().includes(q) || p.invoiceNumber.toLowerCase().includes(q);
      }
      return true;
    });
  }, [payments, search, statusFilter]);

  const pageRows = useMemo(
    () => filtered.slice(pagination.offset, pagination.offset + pagination.pageSize),
    [filtered, pagination.offset, pagination.pageSize]
  );

  const totals = useMemo(() => {
    const list = payments ?? [];
    return {
      due: list.filter((p) => p.status === "Due" || p.status === "Overdue").reduce((s, p) => s + p.netPayable, 0),
      overdue: list.filter((p) => p.status === "Overdue").length,
      tds: list.filter((p) => p.status !== "Paid").reduce((s, p) => s + p.tdsAmount, 0),
    };
  }, [payments]);

  const columns: ColumnDef<PaymentDue>[] = [
    {
      id: "invoiceNumber", header: "Invoice",
      accessorFn: (r) => (
        <button className="text-left cursor-pointer hover:text-primary hover:underline" onClick={() => { setSelected(r); setDialogOpen(true); }}>
          <div className="font-mono text-xs font-bold text-primary">{r.invoiceNumber}</div>
          <div className="text-[10px] font-mono text-slate-400">{formatIndianDate(r.invoiceDate)}</div>
        </button>
      ),
    },
    { id: "vendor", header: "Vendor", accessorFn: (r) => <span className="text-xs font-medium text-slate-700">{r.vendorName}</span> },
    { id: "netPayable", header: "Net Payable (₹)", align: "right", sortable: true, accessorFn: (r) => <span className="font-bold text-slate-800"><MoneyDisplay amount={r.netPayable} /></span>, className: "w-36" },
    {
      id: "tds", header: "TDS (₹)", align: "right",
      accessorFn: (r) => (
        <div className="text-right">
          <p className="font-mono text-xs font-semibold text-rose-600">− <MoneyDisplay amount={r.tdsAmount} /></p>
          <p className="text-[9px] text-slate-400">u/s {r.tdsSection} @{r.tdsRate}%{r.section197 ? " · 197" : ""}</p>
        </div>
      ),
      className: "w-32",
    },
    {
      id: "dueDate", header: "Due Date",
      accessorFn: (r) => (
        <div className="flex items-center gap-1.5">
          <CalendarClock className={`h-3.5 w-3.5 ${r.status === "Overdue" ? "text-rose-500" : "text-slate-400"}`} />
          <span className={`font-mono text-xs ${r.status === "Overdue" ? "font-bold text-rose-600" : "text-slate-600"}`}>{formatIndianDate(r.dueDate)}</span>
        </div>
      ),
    },
    { id: "status", header: "Status", accessorFn: (r) => <StatusBadge status={r.status} />, className: "w-24" },
    {
      id: "actions", header: "", align: "right",
      accessorFn: (r) => (
        <div className="flex justify-end gap-1.5">
          {(r.status === "Due" || r.status === "Overdue") ? (
            <Button size="sm" className="h-7 px-2.5 text-xs bg-emerald-600 text-white hover:bg-emerald-500" onClick={() => { setSelected(r); setDialogOpen(true); }}>
              <Send className="h-3.5 w-3.5 mr-1" /> Pay Now
            </Button>
          ) : (
            <Button variant="outline" size="sm" className="h-7 px-2 text-xs" onClick={() => { setSelected(r); setDialogOpen(true); }}>View</Button>
          )}
        </div>
      ),
      className: "w-28",
    },
  ];

  return (
    <div className="animate-in fade-in duration-500 space-y-6">
      <PageHeader
        title="Payment Processing"
        description="Due vendor payments with TDS deduction at source and bank selection"
        breadcrumbs={[{ label: "Accounts Payable", href: "/ap/vendors" }, { label: "Payments" }]}
        actions={
          <Button size="sm" className="bg-primary hover:bg-primary/90 text-white shadow-lg hover-lift">
            <Banknote className="h-4 w-4 mr-1" /> Schedule Payment
          </Button>
        }
      />

      {/* KPI strip */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Kpi label="Payable Now (Due + Overdue)" value={<MoneyDisplay amount={totals.due} />} tone="text-rose-600" />
        <Kpi label="Overdue Invoices" value={String(totals.overdue)} tone="text-amber-600" />
        <Kpi label="TDS to Deposit (Open)" value={<MoneyDisplay amount={totals.tds} />} tone="text-slate-800" />
      </div>

      <Card className="glass border-white/60 shadow-lg">
        <div className="p-4 border-b border-white/40 bg-white/40 backdrop-blur-md flex flex-wrap items-center gap-3">
          <div className="relative w-80">
            <Search className="absolute left-3 top-2.5 h-4 w-4 text-slate-400" />
            <Input placeholder="Search vendor or invoice..." className="pl-9 h-10 glass-input shadow-sm" value={search} onChange={(e) => setSearch(e.target.value)} />
          </div>
          <select value={statusFilter} onChange={(e) => setStatusFilter(e.target.value)} className="h-10 rounded-lg border border-white/60 bg-white/60 px-3 text-xs font-semibold text-slate-700 shadow-sm focus:outline-none focus:ring-2 focus:ring-primary/40">
            <option value="all">All Statuses</option>
            <option value="Overdue">Overdue</option>
            <option value="Due">Due</option>
            <option value="Scheduled">Scheduled</option>
            <option value="Paid">Paid</option>
          </select>
          <div className="ml-auto text-xs font-bold text-slate-500">{filtered.length} payments</div>
        </div>
        <CardContent className="p-0 custom-scrollbar overflow-auto">
          {isLoading ? (
            <div className="p-6"><TableSkeleton rows={6} cols={7} /></div>
          ) : pageRows.length === 0 ? (
            <div className="flex flex-col items-center justify-center gap-3 py-16 text-center">
              <Banknote className="h-10 w-10 text-slate-300" />
              <p className="text-sm font-bold text-slate-700">{search || statusFilter !== "all" ? "No payments match your filters" : "No payments yet"}</p>
              <p className="text-xs text-slate-400 max-w-sm">
                {search || statusFilter !== "all"
                  ? "Try clearing the search or switching the status filter."
                  : "Approved vendor invoices will appear here as due payments with TDS auto-calculated."}
              </p>
            </div>
          ) : (
            <div className="min-w-[1050px]">
              <DataTable
                data={pageRows}
                columns={columns}
                getRowId={(r) => r.paymentId}
                pagination={{ page: pagination.page, pageSize: pagination.pageSize, total: filtered.length, onPageChange: pagination.setPage, onPageSizeChange: pagination.setPageSize }}
              />
            </div>
          )}
        </CardContent>
      </Card>

      {/* Pay dialog */}
      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-2xl">
          {selected && <PayForm payment={selected} onDone={() => setDialogOpen(false)} />}
        </DialogContent>
      </Dialog>
    </div>
  );
}

function PayForm({ payment, onDone }: { payment: PaymentDue; onDone: () => void }) {
  const banks = MOCK_BANK_ACCOUNTS.filter((b) => b.accountType === "Current");
  const [bankId, setBankId] = useState(payment.bankAccountId ?? banks[0]?.bankAccountId ?? "");
  const [mode, setMode] = useState(payment.mode ?? "NEFT");
  const [payDate, setPayDate] = useState(() => new Date().toISOString().split("T")[0]);
  const [reference, setReference] = useState("");
  const [confirmed, setConfirmed] = useState(false);
  const [paid, setPaid] = useState(false);

  const bank = banks.find((b) => b.bankAccountId === bankId);

  const handlePay = () => {
    setPaid(true);
  };

  return (
    <>
      <DialogHeader>
        <DialogTitle className="flex items-center gap-2 text-lg">
          <Wallet className="h-5 w-5 text-primary" /> Pay {payment.invoiceNumber}
        </DialogTitle>
        <DialogDescription>
          {payment.vendorName} · invoice dated {formatIndianDate(payment.invoiceDate)} · due {formatIndianDate(payment.dueDate)}
        </DialogDescription>
      </DialogHeader>

      {paid ? (
        <div className="flex flex-col items-center gap-3 py-8 text-center">
          <div className="flex h-14 w-14 items-center justify-center rounded-full bg-emerald-100 text-emerald-600">
            <Send className="h-7 w-7" />
          </div>
          <p className="text-sm font-extrabold text-emerald-700">Payment Initiated</p>
          <p className="text-xs text-slate-500 max-w-sm">
            {mode === "Cheque"
              ? `Cheque issued from ${bank?.accountName} in favour of ${payment.vendorName}.`
              : `${mode} transfer of ${(payment.netPayable / 100).toLocaleString("en-IN", { style: "currency", currency: "INR" })} to ${payment.vendorName} queued from ${bank?.accountName}.`}
          </p>
          <p className="font-mono text-[10px] text-slate-400">TDS {payment.tdsSection} of {<MoneyDisplay amount={payment.tdsAmount} />} will be deposited by 7th of next month.</p>
          <Button className="mt-2 bg-primary text-white" onClick={onDone}>Done</Button>
        </div>
      ) : (
        <div className="space-y-4 text-sm">
          {/* TDS calculation preview */}
          <div className="rounded-xl border border-white/50 bg-white/40 p-4">
            <p className="mb-2 flex items-center gap-1.5 text-xs font-bold uppercase tracking-wider text-slate-500">
              <Percent className="h-4 w-4 text-rose-500" /> TDS Auto-Calculation {payment.section197 && <span className="rounded bg-sky-100 px-1.5 py-0.5 text-[9px] text-sky-700">Section 197 — lower rate</span>}
            </p>
            <div className="space-y-1.5 text-xs">
              <div className="flex justify-between text-slate-600"><span>Gross + GST</span><span><MoneyDisplay amount={payment.grossAmount + payment.gstAmount} /></span></div>
              <div className="flex justify-between text-slate-600"><span>TDS u/s {payment.tdsSection} @ {payment.tdsRate}%{payment.section197 ? " (197 certificate applied)" : ""}</span><span className="text-rose-600">− <MoneyDisplay amount={payment.tdsAmount} /></span></div>
              <div className="flex justify-between border-t border-dashed border-white/50 pt-2 text-sm font-extrabold text-emerald-700">
                <span>Net Payable</span><span><MoneyDisplay amount={payment.netPayable} /></span>
              </div>
            </div>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            <div>
              <label className="mb-1 block text-[10px] font-bold uppercase tracking-wider text-slate-500">Pay From (Bank Account)</label>
              <div className="space-y-1.5">
                {banks.map((b) => (
                  <button
                    key={b.bankAccountId}
                    onClick={() => setBankId(b.bankAccountId)}
                    className={`flex w-full items-center justify-between rounded-lg border px-3 py-2 text-left text-xs transition-all ${bankId === b.bankAccountId ? "border-primary bg-primary/5 shadow-sm" : "border-white/60 bg-white/50 hover:border-primary/40"}`}
                  >
                    <span className="font-semibold text-slate-700"><Landmark className="mr-1.5 inline h-3.5 w-3.5 text-slate-400" />{b.accountName}</span>
                    <span className="font-mono text-[10px] text-slate-400">{b.accountNumber.slice(-4).padStart(b.accountNumber.length, "•")} · <MoneyDisplay amount={b.balance} variant="compact" /></span>
                  </button>
                ))}
              </div>
            </div>
            <div className="space-y-3">
              <div>
                <label className="mb-1 block text-[10px] font-bold uppercase tracking-wider text-slate-500">Payment Mode</label>
                <select value={mode} onChange={(e) => setMode(e.target.value)} className="h-10 w-full rounded-lg border border-white/60 bg-white/60 px-3 text-sm font-semibold text-slate-700 focus:outline-none focus:ring-2 focus:ring-primary/40">
                  <option>NEFT</option><option>RTGS</option><option>UPI</option><option>Cheque</option><option>Demand Draft</option>
                </select>
              </div>
              <div>
                <label className="mb-1 block text-[10px] font-bold uppercase tracking-wider text-slate-500">Payment Date</label>
                <Input type="date" value={payDate} onChange={(e) => setPayDate(e.target.value)} className="glass-input" />
              </div>
              <div>
                <label className="mb-1 block text-[10px] font-bold uppercase tracking-wider text-slate-500">Reference (UTR / Cheque No.)</label>
                <Input value={reference} onChange={(e) => setReference(e.target.value)} placeholder="auto-generated if blank" className="glass-input" />
              </div>
            </div>
          </div>

          <label className="flex items-center gap-2 rounded-lg border border-emerald-200 bg-emerald-50/60 px-4 py-3 text-xs font-bold text-emerald-700">
            <input type="checkbox" checked={confirmed} onChange={(e) => setConfirmed(e.target.checked)} className="h-4 w-4" />
            Confirm payment of {<MoneyDisplay amount={payment.netPayable} />} to {payment.vendorName} with TDS {payment.tdsAmount > 0 ? `₹${(payment.tdsAmount / 100).toLocaleString("en-IN")} deducted` : "nil"}.
          </label>
        </div>
      )}

      {!paid && (
        <DialogFooter>
          <Button variant="outline" onClick={onDone}>Cancel</Button>
          <Button className="bg-emerald-600 text-white hover:bg-emerald-500" disabled={!confirmed} onClick={handlePay}>
            <Send className="h-4 w-4 mr-1" /> Initiate Payment
          </Button>
        </DialogFooter>
      )}
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

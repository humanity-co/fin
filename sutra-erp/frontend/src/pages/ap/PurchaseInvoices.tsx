import { useMemo, useState } from "react";
import { Check, FileSearch, Search, ShieldAlert, ShoppingCart, Truck, X } from "lucide-react";
import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button, Input, TableSkeleton, Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "../../components/ui";
import { MoneyDisplay, StatusBadge } from "../../components/data";
import { DataTable, type ColumnDef } from "../../components/data/DataTable";
import { useMockQuery } from "../../hooks/useMockQuery";
import { usePagination } from "../../hooks/usePagination";
import { MOCK_INVOICES, type InvoiceMock } from "../../lib/mock-data";
import { formatIndianDate } from "../../lib/formatters";

/**
 * Invoice Management — vendor invoice register with 3-way matching
 * (PO vs GRN vs Invoice) and approve/reject workflow.
 */
export default function PurchaseInvoices() {
  const pagination = usePagination(10);
  const { data: invoices, isLoading } = useMockQuery<InvoiceMock[]>(MOCK_INVOICES);

  const [search, setSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState("all");
  const [selected, setSelected] = useState<InvoiceMock | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);

  const filtered = useMemo(() => {
    if (!invoices) return [];
    return invoices.filter((inv) => {
      if (statusFilter !== "all" && inv.status !== statusFilter) return false;
      if (search.trim()) {
        const q = search.toLowerCase();
        return (
          inv.invoiceNumber.toLowerCase().includes(q) ||
          inv.vendorName.toLowerCase().includes(q) ||
          (inv.poNumber ?? "").toLowerCase().includes(q)
        );
      }
      return true;
    });
  }, [invoices, search, statusFilter]);

  const pageRows = useMemo(
    () => filtered.slice(pagination.offset, pagination.offset + pagination.pageSize),
    [filtered, pagination.offset, pagination.pageSize]
  );

  const openDetail = (inv: InvoiceMock) => { setSelected(inv); setDialogOpen(true); };

  const matchBadge = (m: InvoiceMock["match"]) => {
    if (m.status === "Full") return <StatusBadge status="Matched" variant="success" />;
    if (m.status === "Partial") return <StatusBadge status="Partial" variant="warning" />;
    return <StatusBadge status="Mismatch" variant="destructive" />;
  };

  const columns: ColumnDef<InvoiceMock>[] = [
    {
      id: "invoiceNumber", header: "Invoice", sortable: true,
      accessorFn: (r) => (
        <button className="text-left cursor-pointer hover:text-primary hover:underline" onClick={() => openDetail(r)}>
          <div className="font-mono text-xs font-bold text-primary">{r.invoiceNumber}</div>
          <div className="text-[10px] font-mono text-slate-400">{formatIndianDate(r.invoiceDate)}</div>
        </button>
      ),
    },
    { id: "vendor", header: "Vendor", accessorFn: (r) => <span className="text-slate-700 text-xs font-medium">{r.vendorName}</span> },
    { id: "po", header: "PO / GRN", accessorFn: (r) => (
      <div className="font-mono text-[11px] text-slate-600">
        <div>{r.poNumber ?? "—"}</div>
        <div className="text-[9px] text-slate-400">{r.grnNumber ?? "no GRN"}</div>
      </div>
    ) },
    { id: "net", header: "Net Amount (₹)", align: "right", accessorFn: (r) => <MoneyDisplay amount={r.netAmount} />, className: "w-32" },
    { id: "match", header: "3-Way Match", accessorFn: (r) => matchBadge(r.match), className: "w-28" },
    { id: "status", header: "Status", accessorFn: (r) => <StatusBadge status={r.status} />, className: "w-28" },
    {
      id: "actions", header: "", align: "right",
      accessorFn: (r) => (
        <div className="flex justify-end gap-1.5">
          <Button variant="outline" size="sm" className="h-7 px-2 text-xs" onClick={() => openDetail(r)}>Match & Review</Button>
        </div>
      ),
      className: "w-32",
    },
  ];

  return (
    <div className="animate-in fade-in duration-500 space-y-6">
      <PageHeader
        title="Purchase Invoices"
        description="Vendor invoice register with 3-way matching (PO vs GRN vs Invoice)"
        breadcrumbs={[{ label: "Accounts Payable", href: "/ap/vendors" }, { label: "Purchase Invoices" }]}
        actions={
          <Button size="sm" className="bg-primary hover:bg-primary/90 text-white shadow-lg hover-lift">
            <FileSearch className="h-4 w-4 mr-1" /> Record Invoice
          </Button>
        }
      />

      <Card className="glass border-white/60 shadow-lg">
        <div className="p-4 border-b border-white/40 bg-white/40 backdrop-blur-md flex flex-wrap items-center gap-3">
          <div className="relative w-80">
            <Search className="absolute left-3 top-2.5 h-4 w-4 text-slate-400" />
            <Input placeholder="Search invoice, vendor, PO..." className="pl-9 h-10 glass-input shadow-sm" value={search} onChange={(e) => setSearch(e.target.value)} />
          </div>
          <select value={statusFilter} onChange={(e) => setStatusFilter(e.target.value)} className="h-10 rounded-lg border border-white/60 bg-white/60 px-3 text-xs font-semibold text-slate-700 shadow-sm focus:outline-none focus:ring-2 focus:ring-primary/40">
            <option value="all">All Statuses</option>
            {["Pending", "Matched", "Approved", "Rejected", "Paid", "PartiallyPaid"].map((s) => <option key={s} value={s}>{s}</option>)}
          </select>
          <div className="ml-auto text-xs font-bold text-slate-500">{filtered.length} invoices</div>
        </div>
        <CardContent className="p-0 custom-scrollbar overflow-auto">
          {isLoading ? (
            <div className="p-6"><TableSkeleton rows={6} cols={7} /></div>
          ) : pageRows.length === 0 ? (
            <div className="flex flex-col items-center justify-center gap-3 py-16 text-center">
              <ShoppingCart className="h-10 w-10 text-slate-300" />
              <p className="text-sm font-bold text-slate-700">{search || statusFilter !== "all" ? "No invoices match your filters" : "No vendor invoices yet"}</p>
              <p className="text-xs text-slate-400 max-w-sm">
                {search || statusFilter !== "all"
                  ? "Try clearing the search or switching the status filter."
                  : "Record a vendor invoice to start the 3-way matching workflow against PO and GRN."}
              </p>
            </div>
          ) : (
            <div className="min-w-[1000px]">
              <DataTable
                data={pageRows}
                columns={columns}
                getRowId={(r) => r.invoiceId}
                pagination={{ page: pagination.page, pageSize: pagination.pageSize, total: filtered.length, onPageChange: pagination.setPage, onPageSizeChange: pagination.setPageSize }}
              />
            </div>
          )}
        </CardContent>
      </Card>

      {/* 3-way matching dialog */}
      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-3xl max-h-[85vh] overflow-y-auto">
          {selected && <MatchingReview invoice={selected} onDone={() => setDialogOpen(false)} />}
        </DialogContent>
      </Dialog>
    </div>
  );
}

function MatchingReview({ invoice, onDone }: { invoice: InvoiceMock; onDone: () => void }) {
  const { match } = invoice;
  const [decision, setDecision] = useState<"approve" | "reject" | null>(null);

  return (
    <>
      <DialogHeader>
        <DialogTitle className="flex items-center gap-2 text-lg">
          <FileSearch className="h-5 w-5 text-primary" /> 3-Way Matching — {invoice.invoiceNumber}
        </DialogTitle>
        <DialogDescription>
          {invoice.vendorName} · {invoice.poNumber ?? "No PO"} · {invoice.grnNumber ?? "No GRN"} · due {formatIndianDate(invoice.dueDate)}
        </DialogDescription>
      </DialogHeader>

      {/* Match status summary */}
      <div className={`flex items-center gap-2 rounded-xl border px-4 py-3 text-xs font-bold ${match.status === "Full" ? "border-emerald-200 bg-emerald-50/80 text-emerald-700" : match.status === "Partial" ? "border-amber-200 bg-amber-50/80 text-amber-700" : "border-rose-200 bg-rose-50/80 text-rose-700"}`}>
        {match.status === "Full" ? <Check className="h-4 w-4" /> : <ShieldAlert className="h-4 w-4" />}
        {match.status === "Full"
          ? "PO, GRN and Invoice quantities & amounts agree — ready to approve."
          : match.status === "Partial"
            ? "Partial match — GRN missing or quantities differ. Review before approval."
            : "Mismatch detected — quantities or amounts differ between PO, GRN and invoice."}
      </div>

      {/* Comparison table */}
      <div className="overflow-hidden rounded-lg border border-white/50">
        <table className="w-full text-left text-xs">
          <thead className="bg-slate-50/70 text-[10px] font-bold uppercase tracking-wider text-slate-500">
            <tr>
              <th className="px-3 py-2">Source</th>
              <th className="px-3 py-2 text-right">Qty</th>
              <th className="px-3 py-2 text-right">Amount (₹)</th>
              <th className="px-3 py-2 text-center">Doc Ref</th>
            </tr>
          </thead>
          <tbody>
            <tr className="border-t border-white/40">
              <td className="px-3 py-2.5 font-semibold text-slate-700"><ShoppingCart className="mr-1 inline h-3.5 w-3.5 text-indigo-500" /> Purchase Order</td>
              <td className="px-3 py-2.5 text-right font-mono">{match.poQty || "—"}</td>
              <td className="px-3 py-2.5 text-right font-mono"><MoneyDisplay amount={match.poAmount} /></td>
              <td className="px-3 py-2.5 text-center font-mono text-slate-500">{invoice.poNumber ?? "—"}</td>
            </tr>
            <tr className="border-t border-white/40">
              <td className="px-3 py-2.5 font-semibold text-slate-700"><Truck className="mr-1 inline h-3.5 w-3.5 text-sky-500" /> Goods Receipt Note</td>
              <td className="px-3 py-2.5 text-right font-mono">{match.grnQty || "—"}</td>
              <td className="px-3 py-2.5 text-right font-mono"><MoneyDisplay amount={match.grnAmount} /></td>
              <td className="px-3 py-2.5 text-center font-mono text-slate-500">{invoice.grnNumber ?? "—"}</td>
            </tr>
            <tr className="border-t border-white/40 bg-white/40">
              <td className="px-3 py-2.5 font-semibold text-slate-800"><FileSearch className="mr-1 inline h-3.5 w-3.5 text-emerald-600" /> Vendor Invoice</td>
              <td className="px-3 py-2.5 text-right font-mono font-bold">{match.invQty}</td>
              <td className="px-3 py-2.5 text-right font-mono font-bold"><MoneyDisplay amount={match.invAmount} /></td>
              <td className="px-3 py-2.5 text-center font-mono text-slate-500">{invoice.invoiceNumber}</td>
            </tr>
          </tbody>
        </table>
      </div>

      <div className="grid grid-cols-3 gap-3 text-xs">
        <div className="rounded-lg border border-white/50 bg-white/40 p-3">
          <p className="text-[10px] font-bold uppercase tracking-wider text-slate-400">Gross Amount</p>
          <p className="mt-0.5 font-bold text-slate-700"><MoneyDisplay amount={invoice.grossAmount} /></p>
        </div>
        <div className="rounded-lg border border-white/50 bg-white/40 p-3">
          <p className="text-[10px] font-bold uppercase tracking-wider text-slate-400">GST</p>
          <p className="mt-0.5 font-bold text-slate-700"><MoneyDisplay amount={invoice.gstAmount} /></p>
        </div>
        <div className="rounded-lg border border-white/50 bg-white/40 p-3">
          <p className="text-[10px] font-bold uppercase tracking-wider text-slate-400">Net Payable</p>
          <p className="mt-0.5 font-extrabold text-emerald-700"><MoneyDisplay amount={invoice.netAmount} /></p>
          {invoice.tdsSection && <p className="text-[9px] text-slate-400">TDS u/s {invoice.tdsSection} at payment</p>}
        </div>
      </div>

      {decision && (
        <div className={`rounded-xl border px-4 py-3 text-xs font-bold ${decision === "approve" ? "border-emerald-200 bg-emerald-50/80 text-emerald-700" : "border-rose-200 bg-rose-50/80 text-rose-700"}`}>
          {decision === "approve"
            ? `Invoice ${invoice.invoiceNumber} approved — queued for payment on ${formatIndianDate(invoice.dueDate)}.`
            : `Invoice ${invoice.invoiceNumber} rejected — vendor will be notified for a credit note.`}
        </div>
      )}

      <DialogFooter>
        <Button variant="outline" onClick={onDone}>Cancel</Button>
        {!decision && (
          <>
            <Button variant="outline" className="border-rose-200 text-rose-600 hover:bg-rose-50" onClick={() => setDecision("reject")}>
              <X className="h-4 w-4 mr-1" /> Reject
            </Button>
            <Button className="bg-emerald-600 text-white hover:bg-emerald-500" disabled={match.status === "Mismatch"} onClick={() => setDecision("approve")}>
              <Check className="h-4 w-4 mr-1" /> Approve for Payment
            </Button>
          </>
        )}
        {decision && <Button className="bg-primary text-white" onClick={onDone}>Done</Button>}
      </DialogFooter>
    </>
  );
}

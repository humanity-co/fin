import { useMemo, useState } from "react";
import { ArrowRight, CheckCircle2, FileText, ListOrdered, Plus, Search, ShieldAlert } from "lucide-react";
import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button, Input, TableSkeleton, Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "../../components/ui";
import { MoneyDisplay, StatusBadge } from "../../components/data";
import { DataTable, type ColumnDef } from "../../components/data/DataTable";
import { useMockQuery } from "../../hooks/useMockQuery";
import { usePagination } from "../../hooks/usePagination";
import { MOCK_PURCHASE_ORDERS, MOCK_VENDORS, type PurchaseOrderMock, type PoLineItem } from "../../lib/mock-data";
import { formatIndianDate } from "../../lib/formatters";

const PO_PIPELINE = ["Draft", "Issued", "Partially Received", "Fully Received", "Closed"];

/**
 * Purchase Orders — PO register with status pipeline
 * (Draft → Issued → Partially Received → Fully Received → Closed)
 * and a create/edit dialog with line items and auto-calculated totals.
 */
export default function PurchaseOrders() {
  const pagination = usePagination(10);
  const { data: orders, isLoading } = useMockQuery<PurchaseOrderMock[]>(MOCK_PURCHASE_ORDERS);

  const [search, setSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState("all");
  const [selected, setSelected] = useState<PurchaseOrderMock | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [creating, setCreating] = useState(false);

  const filtered = useMemo(() => {
    if (!orders) return [];
    return orders.filter((po) => {
      if (statusFilter !== "all" && po.status !== statusFilter) return false;
      if (search.trim()) {
        const q = search.toLowerCase();
        return po.poNumber.toLowerCase().includes(q) || po.vendorName.toLowerCase().includes(q);
      }
      return true;
    });
  }, [orders, search, statusFilter]);

  const pageRows = useMemo(
    () => filtered.slice(pagination.offset, pagination.offset + pagination.pageSize),
    [filtered, pagination.offset, pagination.pageSize]
  );

  const openCreate = () => { setSelected(null); setCreating(true); setDialogOpen(true); };
  const openDetail = (po: PurchaseOrderMock) => { setSelected(po); setCreating(false); setDialogOpen(true); };

  const columns: ColumnDef<PurchaseOrderMock>[] = [
    {
      id: "poNumber", header: "PO Number", sortable: true,
      accessorFn: (r) => (
        <button className="text-left cursor-pointer hover:text-primary hover:underline" onClick={() => openDetail(r)}>
          <div className="font-mono text-xs font-bold text-primary">{r.poNumber}</div>
          <div className="text-[10px] font-mono text-slate-400">{formatIndianDate(r.orderDate)}</div>
        </button>
      ),
    },
    { id: "vendor", header: "Vendor", accessorFn: (r) => <span className="text-xs font-medium text-slate-700">{r.vendorName}</span> },
    {
      id: "status", header: "Pipeline",
      accessorFn: (r) => <PipelineBadge status={r.status} />,
      className: "w-44",
    },
    { id: "net", header: "Net Amount (₹)", align: "right", accessorFn: (r) => <span className="font-bold text-slate-800"><MoneyDisplay amount={r.netAmount} /></span>, className: "w-36" },
    {
      id: "rcm", header: "RCM",
      accessorFn: (r) => r.isRcmApplicable ? <span className="inline-flex items-center gap-1 rounded-full bg-amber-100 px-2 py-0.5 text-[10px] font-bold text-amber-700"><ShieldAlert className="h-3 w-3" /> RCM</span> : <span className="text-slate-300">—</span>,
      className: "w-16",
    },
    { id: "approved", header: "Approved By", accessorFn: (r) => <span className="text-[11px] text-slate-500">{r.approvedBy || <span className="italic text-slate-400">not issued</span>}</span> },
    {
      id: "actions", header: "", align: "right",
      accessorFn: (r) => (
        <div className="flex justify-end gap-1.5">
          <Button variant="outline" size="sm" className="h-7 px-2 text-xs" onClick={() => openDetail(r)}>View</Button>
          {r.status === "Draft" && (
            <Button size="sm" className="h-7 px-2 text-xs bg-primary text-white" onClick={() => openDetail(r)}>Issue</Button>
          )}
        </div>
      ),
      className: "w-28",
    },
  ];

  return (
    <div className="animate-in fade-in duration-500 space-y-6">
      <PageHeader
        title="Purchase Orders"
        description="PO register with status pipeline — draft, issued, received and closed"
        breadcrumbs={[{ label: "Accounts Payable", href: "/ap/vendors" }, { label: "Purchase Orders" }]}
        actions={
          <Button size="sm" className="bg-primary hover:bg-primary/90 text-white shadow-lg hover-lift" onClick={openCreate}>
            <Plus className="h-4 w-4 mr-1" /> Create PO
          </Button>
        }
      />

      <Card className="glass border-white/60 shadow-lg">
        <div className="p-4 border-b border-white/40 bg-white/40 backdrop-blur-md flex flex-wrap items-center gap-3">
          <div className="relative w-80">
            <Search className="absolute left-3 top-2.5 h-4 w-4 text-slate-400" />
            <Input placeholder="Search PO or vendor..." className="pl-9 h-10 glass-input shadow-sm" value={search} onChange={(e) => setSearch(e.target.value)} />
          </div>
          <select value={statusFilter} onChange={(e) => setStatusFilter(e.target.value)} className="h-10 rounded-lg border border-white/60 bg-white/60 px-3 text-xs font-semibold text-slate-700 shadow-sm focus:outline-none focus:ring-2 focus:ring-primary/40">
            <option value="all">All Statuses</option>
            {PO_PIPELINE.map((s) => <option key={s} value={s}>{s}</option>)}
          </select>
          <div className="ml-auto text-xs font-bold text-slate-500">{filtered.length} POs</div>
        </div>
        <CardContent className="p-0 custom-scrollbar overflow-auto">
          {isLoading ? (
            <div className="p-6"><TableSkeleton rows={6} cols={7} /></div>
          ) : pageRows.length === 0 ? (
            <div className="flex flex-col items-center justify-center gap-3 py-16 text-center">
              <ListOrdered className="h-10 w-10 text-slate-300" />
              <p className="text-sm font-bold text-slate-700">{search || statusFilter !== "all" ? "No purchase orders match your filters" : "No purchase orders yet"}</p>
              <p className="text-xs text-slate-400 max-w-sm">
                {search || statusFilter !== "all"
                  ? "Try clearing the search or switching the pipeline status."
                  : "Create your first purchase order to raise procurement requests to onboarded vendors."}
              </p>
              {!search && statusFilter === "all" && (
                <Button size="sm" className="mt-1 bg-primary text-white hover:bg-primary/90" onClick={openCreate}>
                  <Plus className="h-4 w-4 mr-1" /> Create your first →
                </Button>
              )}
            </div>
          ) : (
            <div className="min-w-[1050px]">
              <DataTable
                data={pageRows}
                columns={columns}
                getRowId={(r) => r.purchaseOrderId}
                pagination={{ page: pagination.page, pageSize: pagination.pageSize, total: filtered.length, onPageChange: pagination.setPage, onPageSizeChange: pagination.setPageSize }}
              />
            </div>
          )}
        </CardContent>
      </Card>

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-3xl max-h-[85vh] overflow-y-auto">
          {creating ? (
            <CreatePoForm onDone={() => setDialogOpen(false)} />
          ) : selected ? (
            <PoDetail po={selected} onClose={() => setDialogOpen(false)} />
          ) : null}
        </DialogContent>
      </Dialog>
    </div>
  );
}

/** Mini pipeline indicator for a PO status. */
function PipelineBadge({ status }: { status: string }) {
  const idx = PO_PIPELINE.indexOf(status);
  return (
    <div className="flex items-center gap-1">
      {PO_PIPELINE.map((step, i) => (
        <span key={step} className="flex items-center">
          <span
            className={`h-2 w-2 rounded-full ${i <= idx ? (i < PO_PIPELINE.length - 1 || status === "Closed" ? "bg-emerald-500" : "bg-slate-300") : "bg-slate-200"}`}
            title={step}
          />
          {i < PO_PIPELINE.length - 1 && <span className={`h-0.5 w-3 ${i < idx ? "bg-emerald-400" : "bg-slate-200"}`} />}
        </span>
      ))}
      <StatusBadge status={status} className="ml-2" />
    </div>
  );
}

function PoDetail({ po, onClose }: { po: PurchaseOrderMock; onClose: () => void }) {
  const vendor = MOCK_VENDORS.find((v) => v.vendorId === po.vendorId);
  return (
    <>
      <DialogHeader>
        <DialogTitle className="flex items-center gap-2 text-lg">
          <FileText className="h-5 w-5 text-primary" /> {po.poNumber}
        </DialogTitle>
        <DialogDescription>
          {po.vendorName} · ordered {formatIndianDate(po.orderDate)} · delivery {po.deliveryDate ? formatIndianDate(po.deliveryDate) : "TBD"}
        </DialogDescription>
      </DialogHeader>
      <div className="space-y-4 text-sm">
        <div className="flex flex-wrap items-center gap-3">
          <PipelineBadge status={po.status} />
          <span className="ml-auto"><StatusBadge status={po.isRcmApplicable ? "RCM Applicable" : "No RCM"} variant={po.isRcmApplicable ? "warning" : "secondary"} /></span>
        </div>

        <div className="overflow-hidden rounded-lg border border-white/50">
          <table className="w-full text-left text-xs">
            <thead className="bg-slate-50/70 text-[10px] font-bold uppercase tracking-wider text-slate-500">
              <tr>
                <th className="px-3 py-2">Item</th>
                <th className="px-3 py-2">HSN</th>
                <th className="px-3 py-2 text-right">Qty</th>
                <th className="px-3 py-2 text-right">Rate</th>
                <th className="px-3 py-2 text-right">Amount</th>
                <th className="px-3 py-2 text-right">GST</th>
              </tr>
            </thead>
            <tbody>
              {po.lines.map((l, i) => (
                <tr key={i} className="border-t border-white/40">
                  <td className="px-3 py-2 font-medium text-slate-700">{l.item}</td>
                  <td className="px-3 py-2 font-mono text-slate-500">{l.hsn}</td>
                  <td className="px-3 py-2 text-right font-mono">{l.qty} {l.unit}</td>
                  <td className="px-3 py-2 text-right font-mono"><MoneyDisplay amount={l.rate} /></td>
                  <td className="px-3 py-2 text-right font-semibold"><MoneyDisplay amount={l.amount} /></td>
                  <td className="px-3 py-2 text-right font-mono">{l.gst_amount > 0 ? <MoneyDisplay amount={l.gst_amount} /> : "Exempt"}{l.rcm && <span className="ml-1 text-[9px] font-bold text-amber-600">RCM</span>}</td>
                </tr>
              ))}
            </tbody>
            <tfoot className="bg-slate-50/70">
              <tr>
                <td colSpan={4} className="px-3 py-2 text-right font-bold text-slate-600">Subtotal</td>
                <td className="px-3 py-2 text-right font-bold"><MoneyDisplay amount={po.totalAmount} /></td>
                <td />
              </tr>
              <tr>
                <td colSpan={4} className="px-3 py-2 text-right font-bold text-slate-600">GST</td>
                <td className="px-3 py-2 text-right font-bold"><MoneyDisplay amount={po.taxAmount} /></td>
                <td />
              </tr>
              <tr>
                <td colSpan={4} className="px-3 py-2 text-right font-extrabold text-slate-800">Net Amount</td>
                <td className="px-3 py-2 text-right font-extrabold text-emerald-700"><MoneyDisplay amount={po.netAmount} /></td>
                <td />
              </tr>
            </tfoot>
          </table>
        </div>

        {vendor && (
          <div className="grid grid-cols-2 gap-3 text-xs">
            <div className="rounded-lg border border-white/50 bg-white/40 p-3">
              <p className="text-[10px] font-bold uppercase tracking-wider text-slate-400">Vendor</p>
              <p className="mt-0.5 font-semibold text-slate-700">{vendor.vendorName}</p>
              <p className="font-mono text-[10px] text-slate-400">{vendor.pan}{vendor.gstin ? ` · ${vendor.gstin}` : " · no GSTIN (composition)"}</p>
            </div>
            <div className="rounded-lg border border-white/50 bg-white/40 p-3">
              <p className="text-[10px] font-bold uppercase tracking-wider text-slate-400">Approval</p>
              <p className="mt-0.5 font-semibold text-slate-700">{po.approvedBy || <span className="italic text-slate-400">Awaiting issue & approval</span>}</p>
            </div>
          </div>
        )}
      </div>
      <DialogFooter>
        <Button variant="outline" onClick={onClose}>Close</Button>
        {po.status === "Draft" && <Button className="bg-primary text-white">Issue PO</Button>}
        {(po.status === "Fully Received" || po.status === "Issued") && <Button className="bg-emerald-600 text-white">Close PO</Button>}
      </DialogFooter>
    </>
  );
}

function CreatePoForm({ onDone }: { onDone: () => void }) {
  const [vendorId, setVendorId] = useState("");
  const [lines, setLines] = useState<PoLineItem[]>([
    { item: "", hsn: "9983", qty: 1, unit: "nos", rate: 0, amount: 0, gst_rate: 18, gst_amount: 0, rcm: false },
  ]);

  const updateLine = (i: number, patch: Partial<PoLineItem>) =>
    setLines((prev) =>
      prev.map((l, idx) => {
        if (idx !== i) return l;
        const next = { ...l, ...patch };
        next.amount = next.qty * next.rate;
        next.gst_amount = Math.round(next.amount * (next.gst_rate / 100));
        return next;
      })
    );

  const totals = useMemo(() => {
    const subtotal = lines.reduce((s, l) => s + l.amount, 0);
    const tax = lines.reduce((s, l) => s + l.gst_amount, 0);
    return { subtotal, tax, net: subtotal + tax };
  }, [lines]);

  const handleSave = () => {
    if (!vendorId) { alert("Select a vendor for the PO."); return; }
    const vendor = MOCK_VENDORS.find((v) => v.vendorId === vendorId);
    alert(
      `Purchase order saved as DRAFT.\nVendor: ${vendor?.vendorName}\nNet amount: ${(totals.net / 100).toLocaleString("en-IN", { style: "currency", currency: "INR" })}\nLines: ${lines.length}`
    );
    onDone();
  };

  return (
    <>
      <DialogHeader>
        <DialogTitle className="text-lg">Create Purchase Order</DialogTitle>
        <DialogDescription>Raise a PO with line items — totals auto-calculate with GST and RCM.</DialogDescription>
      </DialogHeader>
      <div className="space-y-4 text-sm">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          <div>
            <label className="mb-1 block text-[10px] font-bold uppercase tracking-wider text-slate-500">Vendor</label>
            <select value={vendorId} onChange={(e) => setVendorId(e.target.value)} className="h-10 w-full rounded-lg border border-white/60 bg-white/60 px-3 text-sm font-semibold text-slate-700 focus:outline-none focus:ring-2 focus:ring-primary/40">
              <option value="">Select vendor...</option>
              {MOCK_VENDORS.filter((v) => v.is_active).map((v) => (
                <option key={v.vendorId} value={v.vendorId}>{v.vendorName} ({v.vendorCode})</option>
              ))}
            </select>
          </div>
          <div>
            <label className="mb-1 block text-[10px] font-bold uppercase tracking-wider text-slate-500">Delivery Expected</label>
            <Input type="date" defaultValue={new Date(Date.now() + 14 * 86400000).toISOString().split("T")[0]} className="glass-input" />
          </div>
        </div>

        <div>
          <label className="mb-2 block text-[10px] font-bold uppercase tracking-wider text-slate-500">Line Items</label>
          <div className="space-y-2">
            {lines.map((l, i) => (
              <div key={i} className="flex flex-wrap items-center gap-2 rounded-lg border border-white/50 bg-white/40 p-2">
                <input value={l.item} onChange={(e) => updateLine(i, { item: e.target.value })} placeholder="Item description" className="h-9 w-48 rounded-md border border-white/60 bg-white/70 px-2 text-xs font-semibold focus:outline-none focus:ring-2 focus:ring-primary/40" />
                <input value={l.hsn} onChange={(e) => updateLine(i, { hsn: e.target.value })} className="h-9 w-20 rounded-md border border-white/60 bg-white/70 px-2 font-mono text-[11px] focus:outline-none" title="HSN/SAC" />
                <Input type="number" value={l.qty || ""} onChange={(e) => updateLine(i, { qty: Number(e.target.value) || 0 })} className="h-9 w-20 glass-input" placeholder="Qty" />
                <Input type="number" value={l.rate || ""} onChange={(e) => updateLine(i, { rate: Math.round(Number(e.target.value) * 100) || 0 })} className="h-9 w-28 glass-input" placeholder="Rate ₹" />
                <select value={l.gst_rate} onChange={(e) => updateLine(i, { gst_rate: Number(e.target.value) })} className="h-9 rounded-md border border-white/60 bg-white/70 px-1 text-[11px] font-semibold focus:outline-none">
                  <option value={0}>GST 0%</option><option value={5}>5%</option><option value={12}>12%</option><option value={18}>18%</option><option value={28}>28%</option>
                </select>
                <label className="flex items-center gap-1 text-[10px] font-bold text-amber-600">
                  <input type="checkbox" checked={l.rcm} onChange={(e) => updateLine(i, { rcm: e.target.checked })} className="h-3.5 w-3.5" /> RCM
                </label>
                <span className="ml-auto font-mono text-xs font-bold text-slate-700"><MoneyDisplay amount={l.amount + l.gst_amount} /></span>
                <button onClick={() => setLines((prev) => prev.filter((_, idx) => idx !== i))} disabled={lines.length <= 1} className="text-slate-400 hover:text-destructive disabled:opacity-30 text-xs font-bold">✕</button>
              </div>
            ))}
          </div>
          <Button variant="outline" size="sm" className="mt-2" onClick={() => setLines((prev) => [...prev, { item: "", hsn: "9983", qty: 1, unit: "nos", rate: 0, amount: 0, gst_rate: 18, gst_amount: 0, rcm: false }])}>
            <Plus className="h-4 w-4 mr-1" /> Add Line Item
          </Button>
        </div>

        <div className="rounded-xl border border-emerald-200 bg-emerald-50/70 p-4 text-xs">
          <div className="flex justify-between font-semibold text-slate-600"><span>Subtotal</span><span><MoneyDisplay amount={totals.subtotal} /></span></div>
          <div className="flex justify-between font-semibold text-slate-600"><span>GST</span><span><MoneyDisplay amount={totals.tax} /></span></div>
          <div className="flex justify-between border-t border-dashed border-emerald-300 pt-1.5 text-sm font-extrabold text-emerald-700"><span>Net Amount</span><span><MoneyDisplay amount={totals.net} /></span></div>
        </div>
      </div>
      <DialogFooter>
        <Button variant="outline" onClick={onDone}>Cancel</Button>
        <Button className="bg-primary text-white" onClick={handleSave}>
          <CheckCircle2 className="h-4 w-4 mr-1" /> Save PO as Draft
        </Button>
      </DialogFooter>
    </>
  );
}

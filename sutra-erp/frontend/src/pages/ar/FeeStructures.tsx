import { useMemo, useState } from "react";
import { BadgePercent, CalendarDays, FileCheck2, GraduationCap, Plus, Search, ShieldCheck } from "lucide-react";
import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button, Input, TableSkeleton, Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "../../components/ui";
import { MoneyDisplay, StatusBadge } from "../../components/data";
import { DataTable, type ColumnDef } from "../../components/data/DataTable";
import { useMockQuery } from "../../hooks/useMockQuery";
import { usePagination } from "../../hooks/usePagination";
import { MOCK_FEE_STRUCTURES, type FeeStructure, type FeeStructureLine } from "../../lib/mock-data";
import { formatIndianDate } from "../../lib/formatters";

/**
 * Fee Structures — per program/batch fee schedules with FRC approval
 * numbers, fee-head breakdowns and installment plans.
 */
export default function FeeStructures() {
  const pagination = usePagination(10);
  const { data: structures, isLoading } = useMockQuery<FeeStructure[]>(MOCK_FEE_STRUCTURES);

  const [search, setSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState("all");
  const [selected, setSelected] = useState<FeeStructure | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [creating, setCreating] = useState(false);

  const filtered = useMemo(() => {
    if (!structures) return [];
    return structures.filter((s) => {
      if (statusFilter !== "all" && s.status !== statusFilter) return false;
      if (search.trim()) {
        const q = search.toLowerCase();
        return s.program.toLowerCase().includes(q) || s.name.toLowerCase().includes(q) || s.frc_approval.toLowerCase().includes(q);
      }
      return true;
    });
  }, [structures, search, statusFilter]);

  const pageRows = useMemo(
    () => filtered.slice(pagination.offset, pagination.offset + pagination.pageSize),
    [filtered, pagination.offset, pagination.pageSize]
  );

  const openCreate = () => { setSelected(null); setCreating(true); setDialogOpen(true); };
  const openDetail = (s: FeeStructure) => { setSelected(s); setCreating(false); setDialogOpen(true); };

  const columns: ColumnDef<FeeStructure>[] = [
    {
      id: "name", header: "Fee Structure", sortable: true,
      accessorFn: (r) => (
        <button className="text-left cursor-pointer hover:text-primary hover:underline" onClick={() => openDetail(r)}>
          <div className="font-semibold text-slate-800">{r.name}</div>
          <div className="text-[10px] font-mono text-slate-400">{r.fee_structure_id.toUpperCase()} · {r.academic_year}</div>
        </button>
      ),
    },
    { id: "program", header: "Program", accessorFn: (r) => <span className="text-slate-600">{r.program}</span> },
    { id: "batch", header: "Batch", accessorFn: (r) => <span className="text-slate-600">{r.batch}</span> },
    {
      id: "total_annual", header: "Annual Fees (₹)", align: "right", sortable: true,
      accessorFn: (r) => <span className="font-bold text-slate-800"><MoneyDisplay amount={r.total_annual} /></span>,
      className: "w-36",
    },
    {
      id: "frc", header: "FRC Approval",
      accessorFn: (r) => (
        <div className="flex items-center gap-1.5">
          <ShieldCheck className="h-3.5 w-3.5 text-emerald-600" />
          <span className="font-mono text-[11px] text-slate-600">{r.frc_approval}</span>
        </div>
      ),
    },
    { id: "status", header: "Status", accessorFn: (r) => <StatusBadge status={r.status} />, className: "w-24" },
    {
      id: "actions", header: "", align: "right",
      accessorFn: (r) => (
        <div className="flex justify-end gap-1.5">
          <Button variant="outline" size="sm" className="h-7 px-2 text-xs" onClick={() => openDetail(r)}>View</Button>
          <Button variant="ghost" size="sm" className="h-7 px-2 text-xs" onClick={() => openDetail(r)}>Edit</Button>
        </div>
      ),
      className: "w-32",
    },
  ];

  return (
    <div className="animate-in fade-in duration-500 space-y-6">
      <PageHeader
        title="Fee Structures"
        description="Per-program fee schedules with FRC approvals and installment plans"
        breadcrumbs={[{ label: "Accounts Receivable" }, { label: "Fee Structures" }]}
        actions={
          <Button size="sm" className="bg-primary hover:bg-primary/90 text-white shadow-lg hover-lift" onClick={openCreate}>
            <Plus className="h-4 w-4 mr-1" /> Create Fee Structure
          </Button>
        }
      />

      <Card className="glass border-white/60 shadow-lg">
        <div className="p-4 border-b border-white/40 bg-white/40 backdrop-blur-md flex flex-wrap items-center gap-3">
          <div className="relative w-80">
            <Search className="absolute left-3 top-2.5 h-4 w-4 text-slate-400" />
            <Input placeholder="Search program, structure, FRC no..." className="pl-9 h-10 glass-input shadow-sm" value={search} onChange={(e) => setSearch(e.target.value)} />
          </div>
          <select value={statusFilter} onChange={(e) => setStatusFilter(e.target.value)} className="h-10 rounded-lg border border-white/60 bg-white/60 px-3 text-xs font-semibold text-slate-700 shadow-sm focus:outline-none focus:ring-2 focus:ring-primary/40">
            <option value="all">All Statuses</option>
            <option value="Active">Active</option>
            <option value="Draft">Draft</option>
            <option value="Archived">Archived</option>
          </select>
          <div className="ml-auto text-xs font-bold text-slate-500">{filtered.length} structures</div>
        </div>
        <CardContent className="p-0 custom-scrollbar overflow-auto">
          {isLoading ? (
            <div className="p-6"><TableSkeleton rows={6} cols={6} /></div>
          ) : pageRows.length === 0 ? (
            <div className="flex flex-col items-center justify-center gap-3 py-16 text-center">
              <GraduationCap className="h-10 w-10 text-slate-300" />
              <p className="text-sm font-bold text-slate-700">{search || statusFilter !== "all" ? "No fee structures match your filters" : "No fee structures yet"}</p>
              <p className="text-xs text-slate-400 max-w-sm">
                {search || statusFilter !== "all"
                  ? "Try clearing the search or switching the status filter."
                  : "Create your first fee structure to define tuition, development and other fee heads per program."}
              </p>
              {!search && statusFilter === "all" && (
                <Button size="sm" className="mt-1 bg-primary text-white hover:bg-primary/90" onClick={openCreate}>
                  <Plus className="h-4 w-4 mr-1" /> Create your first →
                </Button>
              )}
            </div>
          ) : (
            <div className="min-w-[1000px]">
              <DataTable
                data={pageRows}
                columns={columns}
                getRowId={(r) => r.fee_structure_id}
                pagination={{ page: pagination.page, pageSize: pagination.pageSize, total: filtered.length, onPageChange: pagination.setPage, onPageSizeChange: pagination.setPageSize }}
              />
            </div>
          )}
        </CardContent>
      </Card>

      {/* Detail / Create dialog */}
      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-3xl max-h-[85vh] overflow-y-auto">
          {creating ? (
            <CreateFeeStructureForm onDone={() => setDialogOpen(false)} />
          ) : selected ? (
            <FeeStructureDetail structure={selected} onClose={() => setDialogOpen(false)} />
          ) : null}
        </DialogContent>
      </Dialog>
    </div>
  );
}

// ── Detail panel ────────────────────────────────────────────────────────

function FeeStructureDetail({ structure, onClose }: { structure: FeeStructure; onClose: () => void }) {
  const totalRefundable = structure.lines.filter((l) => l.is_refundable).reduce((s, l) => s + l.amount, 0);
  return (
    <>
      <DialogHeader>
        <DialogTitle className="text-lg">{structure.name}</DialogTitle>
        <DialogDescription>
          {structure.program} · {structure.batch} · AY {structure.academic_year}
        </DialogDescription>
      </DialogHeader>
      <div className="space-y-4 text-sm">
        <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
          <Info label="Annual Total" value={<MoneyDisplay amount={structure.total_annual} />} strong />
          <Info label="FRC Approval" value={<span className="font-mono text-[11px]">{structure.frc_approval}</span>} />
          <Info label="Effective From" value={formatIndianDate(structure.effective_from)} />
          <Info label="Status" value={<StatusBadge status={structure.status} />} />
        </div>

        <div>
          <h4 className="mb-2 text-xs font-bold uppercase tracking-wider text-slate-500">Fee Heads ({structure.lines.length})</h4>
          <div className="overflow-hidden rounded-lg border border-white/50">
            <table className="w-full text-left">
              <thead className="bg-slate-50/70 text-[10px] font-bold uppercase tracking-wider text-slate-500">
                <tr>
                  <th className="px-3 py-2">Fee Head</th>
                  <th className="px-3 py-2">Type</th>
                  <th className="px-3 py-2 text-right">Amount</th>
                  <th className="px-3 py-2 text-center">Installments</th>
                  <th className="px-3 py-2 text-center">Refundable</th>
                </tr>
              </thead>
              <tbody>
                {structure.lines.map((l) => (
                  <tr key={l.fee_head} className="border-t border-white/40">
                    <td className="px-3 py-2 font-medium text-slate-700">
                      {l.fee_head}
                      {l.is_optional && <span className="ml-1.5 text-[9px] px-1.5 py-0.5 rounded bg-slate-100 text-slate-500 font-bold">OPTIONAL</span>}
                    </td>
                    <td className="px-3 py-2 text-xs text-slate-500">{l.fee_head_type}</td>
                    <td className="px-3 py-2 text-right font-semibold"><MoneyDisplay amount={l.amount} /></td>
                    <td className="px-3 py-2 text-center">{l.installment_allowed ? <BadgePercent className="mx-auto h-4 w-4 text-emerald-600" /> : "—"}</td>
                    <td className="px-3 py-2 text-center">{l.is_refundable ? <FileCheck2 className="mx-auto h-4 w-4 text-sky-600" /> : "—"}</td>
                  </tr>
                ))}
              </tbody>
              <tfoot className="bg-slate-50/70">
                <tr>
                  <td className="px-3 py-2 font-bold text-slate-700" colSpan={2}>Total (excl. refundable deposit {<MoneyDisplay amount={totalRefundable} />})</td>
                  <td className="px-3 py-2 text-right font-extrabold text-slate-800"><MoneyDisplay amount={structure.total_annual - totalRefundable} /></td>
                  <td colSpan={2} />
                </tr>
              </tfoot>
            </table>
          </div>
        </div>

        <div>
          <h4 className="mb-2 text-xs font-bold uppercase tracking-wider text-slate-500">Installment Plans</h4>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            {structure.installment_plans.map((plan) => (
              <div key={plan.name} className="rounded-lg border border-white/50 bg-white/40 p-3">
                <p className="mb-2 flex items-center gap-1.5 text-xs font-bold text-slate-700">
                  <CalendarDays className="h-3.5 w-3.5 text-primary" /> {plan.name}
                </p>
                {plan.installments.map((inst) => (
                  <div key={inst.number} className="flex items-center justify-between text-xs text-slate-600 py-1">
                    <span>{inst.label} — due {formatIndianDate(inst.due_date)}</span>
                    <span className="font-mono font-bold">{inst.percentage}%</span>
                  </div>
                ))}
              </div>
            ))}
          </div>
        </div>
      </div>
      <DialogFooter>
        <Button variant="outline" onClick={onClose}>Close</Button>
        <Button className="bg-primary text-white">Edit Structure</Button>
      </DialogFooter>
    </>
  );
}

function Info({ label, value, strong }: { label: string; value: React.ReactNode; strong?: boolean }) {
  return (
    <div className="rounded-lg border border-white/50 bg-white/40 p-3">
      <p className="text-[10px] font-bold uppercase tracking-wider text-slate-500">{label}</p>
      <div className={`mt-1 ${strong ? "text-lg font-extrabold text-slate-800" : "text-sm font-semibold text-slate-700"}`}>{value}</div>
    </div>
  );
}

// ── Create form ─────────────────────────────────────────────────────────

const FEE_HEAD_TYPES = ["Tuition", "Development", "Examination", "Library", "Laboratory", "Sports", "Cultural", "Admission", "Registration", "Hostel", "Mess", "Transportation", "CautionDeposit", "Other"];

function CreateFeeStructureForm({ onDone }: { onDone: () => void }) {
  const [name, setName] = useState("");
  const [program, setProgram] = useState("");
  const [frc, setFrc] = useState("");
  const [lines, setLines] = useState<FeeStructureLine[]>([
    { fee_head: "Tuition Fee", fee_head_type: "Tuition", amount: 0, is_optional: false, installment_allowed: true, is_refundable: false },
    { fee_head: "Development Fee", fee_head_type: "Development", amount: 0, is_optional: false, installment_allowed: true, is_refundable: false },
  ]);
  const [installments, setInstallments] = useState("2");

  const total = lines.reduce((s, l) => s + l.amount, 0);

  const updateLine = (i: number, patch: Partial<FeeStructureLine>) =>
    setLines((prev) => prev.map((l, idx) => (idx === i ? { ...l, ...patch } : l)));

  const handleSave = () => {
    // Mock persistence — in production this calls the API.
    alert(
      `Fee structure "${name || program || "Untitled"}" saved as DRAFT.\nFRC approval: ${frc || "to be submitted"}\nAnnual total: ${(total / 100).toLocaleString("en-IN", { style: "currency", currency: "INR", maximumFractionDigits: 0 })}`
    );
    onDone();
  };

  return (
    <>
      <DialogHeader>
        <DialogTitle className="text-lg">Create Fee Structure</DialogTitle>
        <DialogDescription>Define fee heads, amounts and FRC approval for a program/batch.</DialogDescription>
      </DialogHeader>
      <div className="space-y-4 text-sm">
        <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
          <div className="md:col-span-2">
            <Label>Structure Name</Label>
            <Input placeholder="e.g. B.Tech E&TC — AY 2026-27" value={name} onChange={(e) => setName(e.target.value)} className="glass-input" />
          </div>
          <div>
            <Label>Program</Label>
            <select value={program} onChange={(e) => setProgram(e.target.value)} className="h-10 w-full rounded-lg border border-white/60 bg-white/60 px-3 text-sm font-semibold text-slate-700 focus:outline-none focus:ring-2 focus:ring-primary/40">
              <option value="">Select program...</option>
              <option>B.Tech (Computer Science)</option>
              <option>B.Tech (Mechanical)</option>
              <option>B.Com</option>
              <option>B.Sc (Computer Science)</option>
              <option>MBA</option>
              <option>M.Sc (Mathematics)</option>
            </select>
          </div>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          <div>
            <Label>FRC Approval Number</Label>
            <Input placeholder="FRC/SRCOE/2026-27/0XX" value={frc} onChange={(e) => setFrc(e.target.value)} className="glass-input" />
          </div>
          <div>
            <Label>Installment Plan</Label>
            <select value={installments} onChange={(e) => setInstallments(e.target.value)} className="h-10 w-full rounded-lg border border-white/60 bg-white/60 px-3 text-sm font-semibold text-slate-700 focus:outline-none focus:ring-2 focus:ring-primary/40">
              <option value="1">Single Payment</option>
              <option value="2">2 Installments (50/50)</option>
              <option value="4">4 Installments (25% each)</option>
            </select>
          </div>
        </div>

        <div>
          <h4 className="mb-2 text-xs font-bold uppercase tracking-wider text-slate-500">Fee Heads & Amounts</h4>
          <div className="space-y-2">
            {lines.map((l, i) => (
              <div key={i} className="flex flex-wrap items-center gap-2 rounded-lg border border-white/50 bg-white/40 p-2">
                <input
                  value={l.fee_head}
                  onChange={(e) => updateLine(i, { fee_head: e.target.value })}
                  placeholder="Fee head name"
                  className="h-9 w-44 rounded-md border border-white/60 bg-white/70 px-2 text-xs font-semibold text-slate-700 focus:outline-none focus:ring-2 focus:ring-primary/40"
                />
                <select
                  value={l.fee_head_type}
                  onChange={(e) => updateLine(i, { fee_head_type: e.target.value as FeeStructureLine["fee_head_type"] })}
                  className="h-9 rounded-md border border-white/60 bg-white/70 px-2 text-xs font-semibold text-slate-700 focus:outline-none"
                >
                  {FEE_HEAD_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
                </select>
                <Input
                  type="number" placeholder="Amount ₹" value={l.amount || ""}
                  onChange={(e) => updateLine(i, { amount: Math.round(Number(e.target.value) * 100) })}
                  className="h-9 w-36 glass-input"
                />
                <label className="flex items-center gap-1 text-[11px] font-semibold text-slate-600">
                  <input type="checkbox" checked={l.installment_allowed} onChange={(e) => updateLine(i, { installment_allowed: e.target.checked })} className="h-3.5 w-3.5" /> Instal.
                </label>
                <label className="flex items-center gap-1 text-[11px] font-semibold text-slate-600">
                  <input type="checkbox" checked={l.is_refundable} onChange={(e) => updateLine(i, { is_refundable: e.target.checked })} className="h-3.5 w-3.5" /> Refundable
                </label>
                <button
                  onClick={() => setLines((prev) => prev.filter((_, idx) => idx !== i))}
                  disabled={lines.length <= 1}
                  className="ml-auto text-slate-400 hover:text-destructive disabled:opacity-30 text-xs font-bold"
                >
                  Remove
                </button>
              </div>
            ))}
          </div>
          <Button variant="outline" size="sm" className="mt-2" onClick={() => setLines((prev) => [...prev, { fee_head: "", fee_head_type: "Other", amount: 0, is_optional: false, installment_allowed: true, is_refundable: false }])}>
            <Plus className="h-4 w-4 mr-1" /> Add Fee Head
          </Button>
        </div>

        <div className="flex items-center justify-between rounded-lg bg-emerald-50/70 border border-emerald-200 px-4 py-3">
          <span className="text-xs font-bold text-slate-600">Annual Total</span>
          <span className="text-lg font-extrabold text-emerald-700"><MoneyDisplay amount={total} /></span>
        </div>
      </div>
      <DialogFooter>
        <Button variant="outline" onClick={onDone}>Cancel</Button>
        <Button className="bg-emerald-600 text-white hover:bg-emerald-500" onClick={handleSave}>
          Save as Draft
        </Button>
      </DialogFooter>
    </>
  );
}

function Label({ children }: { children: React.ReactNode }) {
  return <label className="mb-1 block text-[10px] font-bold uppercase tracking-wider text-slate-500">{children}</label>;
}

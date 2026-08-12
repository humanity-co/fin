import { useMemo, useState } from "react";
import { useNavigate } from "react-router";
import { FileText, FilterX, Plus, Search } from "lucide-react";
import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button, Input, TableSkeleton } from "../../components/ui";
import { MoneyDisplay, StatusBadge } from "../../components/data";
import { DataTable, type ColumnDef } from "../../components/data/DataTable";
import { useMockQuery } from "../../hooks/useMockQuery";
import { usePagination } from "../../hooks/usePagination";
import { MOCK_JOURNALS, type Journal, type JournalStatus, type JournalType } from "../../lib/mock-data";
import { formatIndianDate } from "../../lib/formatters";

const JOURNAL_TYPES: JournalType[] = ["Standard", "Adjustment", "RCM", "TDS", "Accrual", "Reversing", "ITC Reversal", "Prepayment", "Opening", "Closing"];
const JOURNAL_STATUSES: JournalStatus[] = ["Draft", "Posted", "Reversed", "Cancelled"];

/**
 * Journal List — searchable, filterable register of all journal vouchers.
 * Filters: date range, status, type, amount range. Paginated & sortable.
 */
export default function JournalList() {
  const navigate = useNavigate();
  const pagination = usePagination(10);

  const [search, setSearch] = useState("");
  const [fromDate, setFromDate] = useState("");
  const [toDate, setToDate] = useState("");
  const [status, setStatus] = useState<string>("all");
  const [type, setType] = useState<string>("all");
  const [minAmount, setMinAmount] = useState("");
  const [maxAmount, setMaxAmount] = useState("");

  const { data: journals, isLoading } = useMockQuery<Journal[]>(MOCK_JOURNALS);

  const filtered = useMemo(() => {
    if (!journals) return [];
    return journals
      .filter((j) => {
        if (fromDate && j.journal_date < fromDate) return false;
        if (toDate && j.journal_date > toDate) return false;
        if (status !== "all" && j.status !== status) return false;
        if (type !== "all" && j.journal_type !== type) return false;
        if (minAmount && j.total_amount < Number(minAmount) * 100) return false;
        if (maxAmount && j.total_amount > Number(maxAmount) * 100) return false;
        if (search.trim()) {
          const q = search.toLowerCase();
          return (
            j.journal_number.toLowerCase().includes(q) ||
            j.description.toLowerCase().includes(q) ||
            j.created_by.toLowerCase().includes(q)
          );
        }
        return true;
      })
      .sort((a, b) => b.journal_date.localeCompare(a.journal_date) || b.journal_number.localeCompare(a.journal_number));
  }, [journals, search, fromDate, toDate, status, type, minAmount, maxAmount]);

  const pageRows = useMemo(
    () => filtered.slice(pagination.offset, pagination.offset + pagination.pageSize),
    [filtered, pagination.offset, pagination.pageSize]
  );

  const activeFilters = Boolean(search || fromDate || toDate || status !== "all" || type !== "all" || minAmount || maxAmount);

  const clearFilters = () => {
    setSearch(""); setFromDate(""); setToDate(""); setStatus("all"); setType("all"); setMinAmount(""); setMaxAmount("");
  };

  const columns: ColumnDef<Journal>[] = [
    {
      id: "journal_number", header: "JV Number", sortable: true,
      accessorFn: (r) => <span className="font-mono text-xs font-bold text-primary cursor-pointer hover:underline" onClick={() => navigate(`/gl/journals/${r.journal_id}`)}>{r.journal_number}</span>,
      className: "w-32",
    },
    { id: "journal_date", header: "Date", sortable: true, accessorFn: (r) => <span className="font-mono text-xs text-slate-600">{formatIndianDate(r.journal_date)}</span>, className: "w-28" },
    { id: "journal_type", header: "Type", accessorFn: (r) => <StatusBadge status={r.journal_type} variant={r.journal_type === "RCM" || r.journal_type === "TDS" ? "warning" : "secondary"} />, className: "w-28" },
    { id: "description", header: "Description", accessorFn: (r) => (
      <div>
        <div className="font-medium text-slate-800 line-clamp-1">{r.description}</div>
        {r.reference && <div className="text-[10px] font-mono text-slate-400">Ref: {r.reference}</div>}
      </div>
    ) },
    { id: "total_amount", header: "Amount (₹)", align: "right", sortable: true, accessorFn: (r) => <MoneyDisplay amount={r.total_amount} />, className: "w-36" },
    { id: "status", header: "Status", accessorFn: (r) => <StatusBadge status={r.status} />, className: "w-28" },
    { id: "actions", header: "Actions", align: "center", accessorFn: (r) => (
      <div className="flex items-center justify-end gap-1.5">
        <Button variant="ghost" size="sm" className="h-7 px-2 text-xs" onClick={() => navigate(`/gl/journals/${r.journal_id}`)}>View</Button>
        {r.status === "Draft" && (
          <Button variant="outline" size="sm" className="h-7 px-2 text-xs text-emerald-700 border-emerald-200 hover:bg-emerald-50" onClick={() => navigate(`/gl/journals/${r.journal_id}/post`)}>Post</Button>
        )}
        {r.status === "Posted" && (
          <Button variant="outline" size="sm" className="h-7 px-2 text-xs text-rose-600 border-rose-200 hover:bg-rose-50" onClick={() => navigate(`/gl/journals/${r.journal_id}/reverse`)}>Reverse</Button>
        )}
      </div>
    ), className: "w-44" },
  ];

  return (
    <div className="animate-in fade-in duration-500 space-y-6">
      <PageHeader
        title="Journal Vouchers"
        description="Register of all journal entries — filter, review and post vouchers"
        breadcrumbs={[{ label: "General Ledger" }, { label: "Journals" }]}
        actions={
          <Button size="sm" className="bg-primary hover:bg-primary/90 text-white shadow-lg hover-lift" onClick={() => navigate("/gl/journals/new")}>
            <Plus className="h-4 w-4 mr-1" /> New Journal Entry
          </Button>
        }
      />

      <Card className="glass border-white/60 shadow-lg">
        {/* Filter bar */}
        <div className="p-4 border-b border-white/40 bg-white/40 backdrop-blur-md space-y-3">
          <div className="flex flex-wrap items-center gap-3">
            <div className="relative w-72">
              <Search className="absolute left-3 top-2.5 h-4 w-4 text-slate-400" />
              <Input placeholder="Search JV no, description, creator..." className="pl-9 h-10 glass-input shadow-sm" value={search} onChange={(e) => setSearch(e.target.value)} />
            </div>
            <div className="flex items-center gap-2 text-xs font-semibold text-slate-500">
              <span>From</span>
              <Input type="date" className="h-10 w-38 glass-input shadow-sm" value={fromDate} onChange={(e) => setFromDate(e.target.value)} />
              <span>To</span>
              <Input type="date" className="h-10 w-38 glass-input shadow-sm" value={toDate} onChange={(e) => setToDate(e.target.value)} />
            </div>
          </div>
          <div className="flex flex-wrap items-center gap-3">
            <select value={status} onChange={(e) => setStatus(e.target.value)} className="h-10 rounded-lg border border-white/60 bg-white/60 px-3 text-xs font-semibold text-slate-700 shadow-sm focus:outline-none focus:ring-2 focus:ring-primary/40">
              <option value="all">All Statuses</option>
              {JOURNAL_STATUSES.map((s) => <option key={s} value={s}>{s}</option>)}
            </select>
            <select value={type} onChange={(e) => setType(e.target.value)} className="h-10 rounded-lg border border-white/60 bg-white/60 px-3 text-xs font-semibold text-slate-700 shadow-sm focus:outline-none focus:ring-2 focus:ring-primary/40">
              <option value="all">All Types</option>
              {JOURNAL_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
            </select>
            <div className="flex items-center gap-2 text-xs font-semibold text-slate-500">
              <span>Amount</span>
              <Input type="number" placeholder="Min ₹" className="h-10 w-28 glass-input shadow-sm" value={minAmount} onChange={(e) => setMinAmount(e.target.value)} />
              <span>–</span>
              <Input type="number" placeholder="Max ₹" className="h-10 w-28 glass-input shadow-sm" value={maxAmount} onChange={(e) => setMaxAmount(e.target.value)} />
            </div>
            {activeFilters && (
              <Button variant="outline" size="sm" className="h-10 glass-input" onClick={clearFilters}>
                <FilterX className="h-4 w-4 mr-1" /> Clear
              </Button>
            )}
            <div className="ml-auto text-xs font-bold text-slate-500">{filtered.length} of {journals?.length ?? 0} vouchers</div>
          </div>
        </div>

        <CardContent className="p-0 custom-scrollbar overflow-auto">
          {isLoading ? (
            <div className="p-6"><TableSkeleton rows={8} cols={7} /></div>
          ) : pageRows.length === 0 ? (
            <div className="flex flex-col items-center justify-center gap-3 py-16 text-center">
              <div className="flex h-14 w-14 items-center justify-center rounded-full bg-primary/10">
                <FileText className="h-7 w-7 text-primary" />
              </div>
              <p className="text-sm font-bold text-slate-700">
                {activeFilters ? "No journal entries match your filters" : "No journal entries yet"}
              </p>
              <p className="text-xs text-slate-400 max-w-sm">
                {activeFilters
                  ? "Try widening the date range or clearing the filters."
                  : "Create your first journal entry to start recording vouchers in the general ledger."}
              </p>
              {!activeFilters && (
                <Button size="sm" className="mt-1 bg-primary text-white hover:bg-primary/90" onClick={() => navigate("/gl/journals/new")}>
                  <Plus className="h-4 w-4 mr-1" /> Create your first →
                </Button>
              )}
            </div>
          ) : (
            <div className="min-w-[1000px]">
              <DataTable
                data={pageRows}
                columns={columns}
                getRowId={(r) => r.journal_id}
                pagination={{ page: pagination.page, pageSize: pagination.pageSize, total: filtered.length, onPageChange: pagination.setPage, onPageSizeChange: pagination.setPageSize }}
              />
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

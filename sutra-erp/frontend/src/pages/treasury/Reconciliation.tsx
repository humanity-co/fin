import { useMemo, useState } from "react";
import { CheckCircle2, Scale } from "lucide-react";
import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button, TableSkeleton } from "../../components/ui";
import { MoneyDisplay, StatusBadge } from "../../components/data";
import { DataTable, type ColumnDef } from "../../components/data/DataTable";
import { useMockQuery } from "../../hooks/useMockQuery";
import { usePagination } from "../../hooks/usePagination";
import { MOCK_RECONCILIATION, MOCK_BANK_ACCOUNTS, type ReconciliationRow } from "../../lib/mock-data";
import { formatIndianDate } from "../../lib/formatters";

export default function Reconciliation() {
  const pagination = usePagination(10);
  const { data: rows, isLoading } = useMockQuery<ReconciliationRow[]>(MOCK_RECONCILIATION);
  const [accountFilter, setAccountFilter] = useState("all");

  const filtered = useMemo(() => {
    if (!rows) return [];
    return rows.filter((r) => accountFilter === "all" || r.bankAccountId === accountFilter);
  }, [rows, accountFilter]);

  const pageRows = filtered.slice(pagination.offset, pagination.offset + pagination.pageSize);

  const stats = useMemo(() => {
    const list = filtered;
    return {
      total: list.length,
      matched: list.filter((r) => r.status === "Matched").length,
      unmatched: list.filter((r) => r.status === "Unmatched").length,
      partial: list.filter((r) => r.status === "Partial").length,
    };
  }, [filtered]);

  const columns: ColumnDef<ReconciliationRow>[] = [
    { id: "particular", header: "Particulars", accessorFn: (r) => <span className="text-xs font-medium text-slate-700">{r.particular}</span> },
    { id: "bankRef", header: "Bank Ref / Date", accessorFn: (r) => <div><p className="font-mono text-[11px] text-slate-500">{r.bankRef}</p><p className="text-[10px] font-mono text-slate-400">{formatIndianDate(r.statementDate)}</p></div> },
    { id: "bankAmount", header: "Bank Amount (₹)", align: "right", accessorFn: (r) => <MoneyDisplay amount={r.bankAmount} />, className: "w-32" },
    { id: "systemRef", header: "System Ref", accessorFn: (r) => r.systemRef ? <div><p className="font-mono text-[11px] font-bold text-primary">{r.systemRef}</p><p className="text-[10px] font-mono text-slate-400">{r.systemDate ? formatIndianDate(r.systemDate) : ""}</p></div> : <span className="text-slate-300">—</span> },
    { id: "systemAmount", header: "System Amount (₹)", align: "right", accessorFn: (r) => r.systemAmount != null ? <MoneyDisplay amount={r.systemAmount} /> : <span className="text-slate-300">—</span>, className: "w-32" },
    { id: "status", header: "Status", accessorFn: (r) => <StatusBadge status={r.status} />, className: "w-28" },
  ];

  return (
    <div className="animate-in fade-in duration-500 space-y-6">
      <PageHeader
        title="Bank Reconciliation"
        description="Match bank statement lines against system transactions"
        breadcrumbs={[{ label: "Treasury" }, { label: "Reconciliation" }]}
        actions={<Button size="sm" className="bg-primary hover:bg-primary/90 text-white shadow-lg hover-lift">Start Reconciliation</Button>}
      />
      {/* Stat pills */}
      <div className="flex flex-wrap gap-3">
        <Pill label="Total" value={stats.total} className="bg-white/60 text-slate-700" />
        <Pill label="Matched ✓" value={stats.matched} className="bg-emerald-50 text-emerald-700 border-emerald-200" />
        <Pill label="Unmatched ✗" value={stats.unmatched} className="bg-rose-50 text-rose-700 border-rose-200" />
        <Pill label="Partial ⚠" value={stats.partial} className="bg-amber-50 text-amber-700 border-amber-200" />
        <div className="ml-auto">
          <select value={accountFilter} onChange={(e) => setAccountFilter(e.target.value)} className="h-9 rounded-lg border border-white/60 bg-white/60 px-3 text-xs font-semibold text-slate-700 focus:outline-none">
            <option value="all">All Accounts</option>
            {MOCK_BANK_ACCOUNTS.map((b) => <option key={b.bankAccountId} value={b.bankAccountId}>{b.accountName}</option>)}
          </select>
        </div>
      </div>
      <Card className="glass border-white/60 shadow-lg">
        <CardContent className="p-0 custom-scrollbar overflow-auto">
          {isLoading ? <div className="p-6"><TableSkeleton rows={7} cols={6} /></div>
          : pageRows.length === 0 ? (
            <div className="flex flex-col items-center justify-center gap-2 py-16 text-center">
              <Scale className="h-10 w-10 text-slate-300" />
              <p className="text-sm font-semibold text-slate-600">No statement lines for this account</p>
              <p className="text-xs text-slate-400">Start a reconciliation to import the bank statement and match transactions.</p>
            </div>
          ) : (
            <div className="min-w-[900px]">
              <DataTable data={pageRows} columns={columns} getRowId={(r) => r.reconciliationId}
                pagination={{ page: pagination.page, pageSize: pagination.pageSize, total: filtered.length, onPageChange: pagination.setPage, onPageSizeChange: pagination.setPageSize }} />
              {stats.unmatched === 0 && stats.partial === 0 && (
                <div className="flex items-center gap-2 border-t border-white/40 bg-emerald-50/60 px-6 py-3 text-xs font-bold text-emerald-700">
                  <CheckCircle2 className="h-4 w-4" /> All statement lines matched — {stats.total} of {stats.total}
                </div>
              )}
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

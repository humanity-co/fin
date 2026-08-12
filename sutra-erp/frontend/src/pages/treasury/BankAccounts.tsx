import { useMemo, useState } from "react";
import { Landmark, Plus, Search } from "lucide-react";
import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button, Input, TableSkeleton } from "../../components/ui";
import { MoneyDisplay, StatusBadge } from "../../components/data";
import { DataTable, type ColumnDef } from "../../components/data/DataTable";
import { useMockQuery } from "../../hooks/useMockQuery";
import { usePagination } from "../../hooks/usePagination";
import { MOCK_BANK_ACCOUNTS, type BankAccountMock } from "../../lib/mock-data";
import { formatIndianDate } from "../../lib/formatters";

export default function BankAccounts() {
  const pagination = usePagination(10);
  const { data: accounts, isLoading } = useMockQuery<BankAccountMock[]>(MOCK_BANK_ACCOUNTS);
  const [search, setSearch] = useState("");

  const filtered = useMemo(() => {
    if (!accounts) return [];
    const q = search.toLowerCase();
    return accounts.filter((a) => !q || a.accountName.toLowerCase().includes(q) || a.bankName.toLowerCase().includes(q) || a.ifsc.toLowerCase().includes(q));
  }, [accounts, search]);

  const pageRows = filtered.slice(pagination.offset, pagination.offset + pagination.pageSize);
  const totalBalance = useMemo(() => (accounts ?? []).reduce((s, a) => s + a.balance, 0), [accounts]);

  const columns: ColumnDef<BankAccountMock>[] = [
    {
      id: "accountName", header: "Account",
      accessorFn: (r) => (
        <div>
          <p className="font-semibold text-slate-800">{r.accountName} {r.isPrimary && <span className="ml-1 rounded bg-primary/10 px-1.5 py-0.5 text-[9px] font-bold text-primary">PRIMARY</span>}</p>
          <p className="font-mono text-[10px] text-slate-400">{r.accountNumber}</p>
        </div>
      ),
    },
    { id: "bank", header: "Bank & Branch", accessorFn: (r) => <div><p className="text-xs font-medium text-slate-600">{r.bankName}</p><p className="text-[10px] text-slate-400">{r.branch}</p></div> },
    { id: "ifsc", header: "IFSC", accessorFn: (r) => <span className="font-mono text-[11px] text-slate-600">{r.ifsc}</span> },
    { id: "type", header: "Type", accessorFn: (r) => <StatusBadge status={r.accountType} variant={r.accountType === "Current" ? "info" : "secondary"} /> },
    { id: "balance", header: "Balance (₹)", align: "right", accessorFn: (r) => <span className="font-bold text-slate-800"><MoneyDisplay amount={r.balance} /></span>, className: "w-36" },
    { id: "purpose", header: "Purpose", accessorFn: (r) => <span className="text-[11px] text-slate-500">{r.purpose}</span> },
    { id: "recon", header: "Last Reconciled", accessorFn: (r) => <span className="font-mono text-[11px] text-slate-500">{formatIndianDate(r.lastReconciled)}</span>, className: "w-28" },
  ];

  return (
    <div className="animate-in fade-in duration-500 space-y-6">
      <PageHeader
        title="Bank Accounts"
        description="Linked bank accounts with IFSC, balances and reconciliation status"
        breadcrumbs={[{ label: "Treasury" }, { label: "Bank Accounts" }]}
        actions={<Button size="sm" className="bg-primary hover:bg-primary/90 text-white shadow-lg hover-lift"><Plus className="h-4 w-4 mr-1" /> Add Bank Account</Button>}
      />
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Kpi label="Linked Accounts" value={String(accounts?.length ?? 0)} />
        <Kpi label="Total Balance" value={<MoneyDisplay amount={totalBalance} />} />
        <Kpi label="Primary Operating A/c" value={<span className="text-sm font-bold text-slate-600">{MOCK_BANK_ACCOUNTS.find((b) => b.isPrimary)?.accountName}</span>} />
      </div>
      <Card className="glass border-white/60 shadow-lg">
        <div className="p-4 border-b border-white/40 bg-white/40 backdrop-blur-md flex items-center gap-3">
          <div className="relative w-80">
            <Search className="absolute left-3 top-2.5 h-4 w-4 text-slate-400" />
            <Input placeholder="Search account, bank, IFSC..." className="pl-9 h-10 glass-input shadow-sm" value={search} onChange={(e) => setSearch(e.target.value)} />
          </div>
          <div className="ml-auto text-xs font-bold text-slate-500">{filtered.length} accounts</div>
        </div>
        <CardContent className="p-0 custom-scrollbar overflow-auto">
          {isLoading ? <div className="p-6"><TableSkeleton rows={5} cols={6} /></div>
          : pageRows.length === 0 ? (
            <div className="flex flex-col items-center justify-center gap-2 py-16 text-center">
              <Landmark className="h-10 w-10 text-slate-300" />
              <p className="text-sm font-semibold text-slate-600">{search ? "No bank accounts match your search" : "No bank accounts linked yet"}</p>
              <p className="text-xs text-slate-400">Link a bank account to start reconciling statements.</p>
            </div>
          ) : (
            <div className="min-w-[950px]">
              <DataTable data={pageRows} columns={columns} getRowId={(r) => r.bankAccountId}
                pagination={{ page: pagination.page, pageSize: pagination.pageSize, total: filtered.length, onPageChange: pagination.setPage, onPageSizeChange: pagination.setPageSize }} />
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function Kpi({ label, value }: { label: string; value: React.ReactNode }) {
  return <Card className="glass border-white/60 p-4 shadow-sm"><p className="text-[10px] font-bold uppercase tracking-widest text-slate-500">{label}</p><p className="mt-1 text-xl font-extrabold text-slate-800">{value}</p></Card>;
}

import { useMemo, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { ArrowLeft, Download, FileText, Search } from "lucide-react";
import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button, Input, TableSkeleton } from "../../components/ui";
import { MoneyDisplay, StatusBadge } from "../../components/data";
import { DataTable, type ColumnDef } from "../../components/data/DataTable";
import { useMockQuery } from "../../hooks/useMockQuery";
import { COA_ACCOUNTS, MOCK_LEDGER, generateLedger, type LedgerEntry } from "../../lib/mock-data";
import { formatIndianDate, formatIndianNumber } from "../../lib/formatters";

/**
 * Account Ledger — dated transaction history with running balance for a
 * single GL account. Reachable from Chart of Accounts & Trial Balance.
 */
export default function AccountLedger() {
  const { accountId } = useParams<{ accountId: string }>();
  const account = COA_ACCOUNTS.find((a) => a.account_id === accountId) ?? COA_ACCOUNTS[3];

  const [fromDate, setFromDate] = useState("");
  const [toDate, setToDate] = useState("");
  const [search, setSearch] = useState("");

  const { data: allEntries, isLoading } = useMockQuery<LedgerEntry[]>(
    MOCK_LEDGER[account.account_id] ?? generateLedger(account)
  );

  const entries = useMemo(() => {
    if (!allEntries) return [];
    return allEntries
      .filter((e) => {
        if (fromDate && e.date < fromDate) return false;
        if (toDate && e.date > toDate) return false;
        if (search.trim()) {
          const q = search.toLowerCase();
          return (
            e.voucher.toLowerCase().includes(q) ||
            e.particular.toLowerCase().includes(q)
          );
        }
        return true;
      })
      .sort((a, b) => a.date.localeCompare(b.date) || a.entry_id.localeCompare(b.entry_id));
  }, [allEntries, fromDate, toDate, search]);

  // Recompute running balance from the opening balance so filters stay correct.
  const opening = Math.abs(account.opening_balance ?? 0) * (account.account_type === "Asset" || account.account_type === "Expense" ? 1 : -1);
  const withBalance = useMemo(() => {
    let running = opening;
    return entries.map((e) => {
      running += e.debit - e.credit;
      return { ...e, balance: running };
    });
  }, [entries, opening]);

  const totals = useMemo(
    () => ({
      debit: withBalance.reduce((s, e) => s + e.debit, 0),
      credit: withBalance.reduce((s, e) => s + e.credit, 0),
      closing: opening + withBalance.reduce((s, e) => s + e.debit - e.credit, 0),
    }),
    [withBalance, opening]
  );

  const columns: ColumnDef<LedgerEntry & { balance: number }>[] = [
    { id: "date", header: "Date", accessorFn: (r) => <span className="font-mono text-xs text-slate-600">{formatIndianDate(r.date)}</span>, className: "w-28" },
    { id: "voucher", header: "Voucher", accessorFn: (r) => <span className="font-mono text-xs font-semibold text-primary">{r.voucher}</span>, className: "w-36" },
    { id: "particular", header: "Particulars", accessorFn: (r) => r.particular },
    { id: "debit", header: "Debit (₹)", align: "right", accessorFn: (r) => r.debit > 0 ? <MoneyDisplay amount={r.debit} /> : "—", className: "w-40" },
    { id: "credit", header: "Credit (₹)", align: "right", accessorFn: (r) => r.credit > 0 ? <MoneyDisplay amount={r.credit} /> : "—", className: "w-40" },
    {
      id: "balance", header: "Running Balance (₹)", align: "right", sortable: true,
      accessorFn: (r) => (
        <span className={r.balance < 0 ? "text-destructive font-semibold" : "font-semibold text-slate-800"}>
          {Math.abs(r.balance) > 0 ? (
            <MoneyDisplay amount={r.balance} variant="accounting" />
          ) : "—"}
        </span>
      ),
      className: "w-44",
    },
  ];

  return (
    <div className="animate-in fade-in duration-500 space-y-6">
      <PageHeader
        title={account.account_name}
        description={`Ledger · ${account.account_code} · ${account.account_type}`}
        breadcrumbs={[
          { label: "General Ledger", href: "/gl/accounts" },
          { label: "Chart of Accounts", href: "/gl/accounts" },
          { label: account.account_name },
        ]}
        actions={
          <div className="flex gap-2">
            <Button variant="outline" size="sm" className="glass-input hover-lift">
              <Download className="h-4 w-4 mr-2" /> Export
            </Button>
            <Link to="/gl/accounts">
              <Button variant="outline" size="sm" className="glass-input hover-lift">
                <ArrowLeft className="h-4 w-4 mr-2" /> Chart of Accounts
              </Button>
            </Link>
          </div>
        }
      />

      {/* Summary strip */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        {[
          { label: "Opening Balance", value: opening, hint: "as on 01-Apr-2026" },
          { label: "Total Debits (period)", value: totals.debit, hint: `${entries.length} entries` },
          { label: "Total Credits (period)", value: totals.credit, hint: `${entries.length} entries` },
          { label: "Closing Balance", value: totals.closing, hint: "as on 30-Jun-2026", strong: true },
        ].map((kpi) => (
          <Card key={kpi.label} className="glass border-white/60 p-4 shadow-sm">
            <p className="text-[10px] font-bold uppercase tracking-widest text-slate-500">{kpi.label}</p>
            <div className={`mt-1 ${kpi.strong ? "text-lg" : "text-base"} font-extrabold ${totals.closing < 0 ? "text-destructive" : "text-slate-800"}`}>
              <MoneyDisplay amount={kpi.value} variant="accounting" />
            </div>
            <p className="text-[10px] text-slate-400 font-medium">{kpi.hint}</p>
          </Card>
        ))}
      </div>

      <Card className="glass border-white/60 shadow-lg">
        {/* Filters */}
        <div className="p-4 border-b border-white/40 bg-white/40 backdrop-blur-md flex flex-wrap items-center gap-3">
          <div className="relative w-72">
            <Search className="absolute left-3 top-2.5 h-4 w-4 text-slate-400" />
            <Input
              placeholder="Search voucher or particulars..."
              className="pl-9 h-10 glass-input shadow-sm"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
          </div>
          <div className="flex items-center gap-2 text-xs font-semibold text-slate-500">
            <FileText className="h-4 w-4" /> From
            <Input type="date" className="h-10 w-40 glass-input shadow-sm" value={fromDate} onChange={(e) => setFromDate(e.target.value)} />
            <span>To</span>
            <Input type="date" className="h-10 w-40 glass-input shadow-sm" value={toDate} onChange={(e) => setToDate(e.target.value)} />
          </div>
          <div className="ml-auto flex items-center gap-2">
            <span className="text-[10px] font-bold uppercase tracking-wider text-slate-500">Nature</span>
            <StatusBadge status={account.account_type} />
            {account.gst_applicable && <StatusBadge status="GST" variant="info" />}
          </div>
        </div>
        <CardContent className="p-0 custom-scrollbar overflow-auto">
          {isLoading ? (
            <div className="p-6">
              <TableSkeleton rows={6} cols={5} />
            </div>
          ) : withBalance.length === 0 ? (
            <div className="flex flex-col items-center justify-center gap-2 py-16 text-center">
              <FileText className="h-10 w-10 text-slate-300" />
              <p className="text-sm font-semibold text-slate-600">No transactions found</p>
              <p className="text-xs text-slate-400">Adjust the date range or search to see entries for {account.account_name}.</p>
            </div>
          ) : (
            <div className="min-w-[900px]">
              <DataTable
                data={withBalance}
                columns={columns}
                getRowId={(r) => r.entry_id}
                pagination={{
                  page: 1,
                  pageSize: 20,
                  total: withBalance.length,
                  onPageChange: () => undefined,
                  onPageSizeChange: () => undefined,
                }}
              />
              {/* Totals row */}
              <div className="flex flex-wrap items-center justify-end gap-x-10 gap-y-1 border-t-2 border-slate-200 bg-white/60 px-6 py-3 text-xs font-extrabold text-slate-700">
                <span>Opening: <MoneyDisplay amount={opening} /></span>
                <span>Debit Total: <MoneyDisplay amount={totals.debit} /></span>
                <span>Credit Total: <MoneyDisplay amount={totals.credit} /></span>
                <span>Closing: <MoneyDisplay amount={totals.closing} variant="accounting" /></span>
                <span className="text-[10px] font-semibold text-slate-400">({formatIndianNumber(withBalance.length)} entries)</span>
              </div>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

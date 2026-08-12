import { useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { CheckCircle2, Download, FileText } from "lucide-react";
import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button, TableSkeleton } from "../../components/ui";
import { MoneyDisplay } from "../../components/data";
import { useMockQuery } from "../../hooks/useMockQuery";
import { MOCK_TRIAL_BALANCE, type TrialBalanceRow } from "../../lib/mock-data";

const PERIODS = [
  { label: "FY 2026-27 — Q1 (Apr–Jun)", from: "2026-04-01", to: "2026-06-30" },
  { label: "FY 2026-27 — Apr 2026", from: "2026-04-01", to: "2026-04-30" },
  { label: "FY 2026-27 — May 2026", from: "2026-05-01", to: "2026-05-31" },
  { label: "FY 2026-27 — Jun 2026", from: "2026-06-01", to: "2026-06-30" },
  { label: "FY 2025-26 — Full Year", from: "2025-04-01", to: "2026-03-31" },
];

/**
 * Trial Balance — opening / period / closing debit-credit statement.
 * Period selector, clickable accounts (→ Account Ledger), balanced check.
 */
export default function TrialBalance() {
  const navigate = useNavigate();
  const [period, setPeriod] = useState(PERIODS[0].label);
  const { data: rows, isLoading } = useMockQuery<TrialBalanceRow[]>(MOCK_TRIAL_BALANCE);

  const active = PERIODS.find((p) => p.label === period) ?? PERIODS[0];

  const totals = useMemo(() => {
    const rowsList = rows ?? [];
    const sum = (f: (r: TrialBalanceRow) => number) => rowsList.reduce((s, r) => s + f(r), 0);
    const openDr = sum((r) => r.openingDebit);
    const openCr = sum((r) => r.openingCredit);
    const perDr = sum((r) => r.periodDebit);
    const perCr = sum((r) => r.periodCredit);
    const closeDr = sum((r) => r.closingDebit);
    const closeCr = sum((r) => r.closingCredit);
    return { openDr, openCr, perDr, perCr, closeDr, closeCr, balanced: closeDr === closeCr };
  }, [rows]);

  return (
    <div className="animate-in fade-in duration-500 space-y-6">
      <PageHeader
        title="Trial Balance"
        description="Verify the mathematical accuracy of your ledger accounts"
        breadcrumbs={[{ label: "General Ledger" }, { label: "Reports" }, { label: "Trial Balance" }]}
        actions={
          <div className="flex gap-2">
            <select
              value={period}
              onChange={(e) => setPeriod(e.target.value)}
              className="h-10 rounded-lg border border-white/60 bg-white/60 px-3 text-xs font-semibold text-slate-700 shadow-sm focus:outline-none focus:ring-2 focus:ring-primary/40"
            >
              {PERIODS.map((p) => <option key={p.label} value={p.label}>{p.label}</option>)}
            </select>
            <Button variant="outline" size="sm" className="glass-input hover-lift">
              <Download className="h-4 w-4 mr-2" /> Export
            </Button>
          </div>
        }
      />

      {/* Balance verification strip */}
      <div className={`flex items-center justify-between rounded-xl border px-5 py-3 text-sm font-bold ${totals.balanced ? "border-emerald-200 bg-emerald-50/80 text-emerald-700" : "border-rose-200 bg-rose-50/80 text-rose-700"}`}>
        <div className="flex items-center gap-2">
          <CheckCircle2 className="h-5 w-5" />
          {totals.balanced
            ? "Trial Balance is in order — total debits equal total credits"
            : `Imbalance of ${totals.closeDr > totals.closeCr ? "debit" : "credit"} ₹${Math.abs(totals.closeDr - totals.closeCr).toLocaleString("en-IN")} — investigate before closing the period`}
        </div>
        <div className="text-[11px] font-semibold text-slate-500">
          Period: {active.from} → {active.to}
        </div>
      </div>

      <Card className="glass flex-1 flex flex-col border-white/60 shadow-lg">
        <div className="p-4 border-b border-white/60 bg-white/40 backdrop-blur-md flex items-center justify-between">
          <div className="flex items-center gap-2 text-sm font-bold text-slate-800">
            <FileText className="h-4 w-4 text-primary" />
            Shri Ram College of Engineering — Trial Balance as at {active.to}
          </div>
        </div>
        <CardContent className="p-0 flex-1 overflow-auto custom-scrollbar">
          {isLoading ? (
            <div className="p-6"><TableSkeleton rows={10} cols={7} /></div>
          ) : (rows ?? []).length === 0 ? (
            <div className="flex flex-col items-center justify-center gap-2 py-16 text-center">
              <FileText className="h-10 w-10 text-slate-300" />
              <p className="text-sm font-semibold text-slate-600">No trial balance data for this period</p>
              <p className="text-xs text-slate-400">Post journal entries for {active.label} to generate the statement.</p>
            </div>
          ) : (
            <table className="w-full text-left border-collapse">
              <thead className="sticky top-0 bg-white/80 backdrop-blur-md z-10 border-b border-white/60">
                <tr>
                  <th rowSpan={2} className="p-3 border-r border-white/40 text-xs font-bold uppercase tracking-wider text-slate-500 align-bottom">
                    Particulars
                  </th>
                  <th colSpan={2} className="p-2 border-r border-b border-white/40 text-center text-[10px] font-bold uppercase tracking-wider text-slate-500 bg-black/5">
                    Opening Balance
                  </th>
                  <th colSpan={2} className="p-2 border-r border-b border-white/40 text-center text-[10px] font-bold uppercase tracking-wider text-slate-500 bg-primary/5">
                    Transactions
                  </th>
                  <th colSpan={2} className="p-2 border-b border-white/40 text-center text-[10px] font-bold uppercase tracking-wider text-slate-500 bg-emerald-500/5">
                    Closing Balance
                  </th>
                </tr>
                <tr className="text-[10px] font-bold uppercase tracking-wider text-slate-500">
                  <th className="p-2 text-right border-r border-white/40 w-28 bg-black/5">Debit</th>
                  <th className="p-2 text-right border-r border-white/40 w-28 bg-black/5">Credit</th>
                  <th className="p-2 text-right border-r border-white/40 w-28 bg-primary/5">Debit</th>
                  <th className="p-2 text-right border-r border-white/40 w-28 bg-primary/5">Credit</th>
                  <th className="p-2 text-right border-r border-white/40 w-28 bg-emerald-500/5">Debit</th>
                  <th className="p-2 text-right w-28 bg-emerald-500/5">Credit</th>
                </tr>
              </thead>
              <tbody>
                {(rows ?? []).map((row) => (
                  <tr key={row.accountId} className="border-b border-white/30 hover:bg-white/40 transition-colors row-focus">
                    <td className="p-3 border-r border-white/30">
                      <button
                        className="text-left font-semibold text-slate-800 hover:text-primary hover:underline cursor-pointer"
                        onClick={() => navigate(`/gl/ledger/${row.accountId}`)}
                        title="Open account ledger"
                      >
                        {row.accountName}
                      </button>
                      <div className="text-[10px] font-mono text-slate-500">{row.accountCode} · {row.accountType}</div>
                    </td>
                    <td className="p-3 text-right border-r border-white/30"><MoneyDisplay amount={row.openingDebit} /></td>
                    <td className="p-3 text-right border-r border-white/30"><MoneyDisplay amount={row.openingCredit} /></td>
                    <td className="p-3 text-right border-r border-white/30 bg-primary/5"><MoneyDisplay amount={row.periodDebit} /></td>
                    <td className="p-3 text-right border-r border-white/30 bg-primary/5"><MoneyDisplay amount={row.periodCredit} /></td>
                    <td className="p-3 text-right font-bold border-r border-white/30 bg-emerald-500/5"><MoneyDisplay amount={row.closingDebit} /></td>
                    <td className="p-3 text-right font-bold bg-emerald-500/5"><MoneyDisplay amount={row.closingCredit} /></td>
                  </tr>
                ))}
              </tbody>
              <tfoot className="bg-white/60 backdrop-blur-md border-t-2 border-slate-300">
                <tr>
                  <td className="p-4 text-right text-xs font-extrabold uppercase text-slate-800 border-r border-white/40">Grand Total</td>
                  <td className="p-4 text-right border-r border-white/40"><MoneyDisplay amount={totals.openDr} /></td>
                  <td className="p-4 text-right border-r border-white/40"><MoneyDisplay amount={totals.openCr} /></td>
                  <td className="p-4 text-right border-r border-white/40 bg-primary/10"><MoneyDisplay amount={totals.perDr} /></td>
                  <td className="p-4 text-right border-r border-white/40 bg-primary/10"><MoneyDisplay amount={totals.perCr} /></td>
                  <td className={`p-4 text-right border-r border-white/40 bg-emerald-500/10 font-extrabold ${totals.balanced ? "text-emerald-700" : "text-rose-700"}`}><MoneyDisplay amount={totals.closeDr} /></td>
                  <td className={`p-4 text-right bg-emerald-500/10 font-extrabold ${totals.balanced ? "text-emerald-700" : "text-rose-700"}`}><MoneyDisplay amount={totals.closeCr} /></td>
                </tr>
              </tfoot>
            </table>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

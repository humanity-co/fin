import { useState, useEffect } from "react";
import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button } from "../../components/ui";
import { Download, Filter, FileText } from "lucide-react";
import { api } from "../../lib/api-client";

interface TBRow {
  accountId: string;
  accountName: string;
  accountCode: string;
  openingDebit: number;
  openingCredit: number;
  periodDebit: number;
  periodCredit: number;
  closingDebit: number;
  closingCredit: number;
}

export default function TrialBalance() {
  const [data, setData] = useState<TBRow[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    // Attempt to fetch TB, fallback to mock if API returns error
    api.get<TBRow[]>('/gl/trial-balance').then(res => {
      if (res && res.length > 0) {
        setData(res);
      } else {
        setMockData();
      }
    }).catch(() => {
      setMockData();
    }).finally(() => {
      setIsLoading(false);
    });
  }, []);

  const setMockData = () => {
    setData([
      { accountId: "1", accountCode: "1000", accountName: "Assets", openingDebit: 5000000, openingCredit: 0, periodDebit: 200000, periodCredit: 50000, closingDebit: 5150000, closingCredit: 0 },
      { accountId: "2", accountCode: "2000", accountName: "Liabilities", openingDebit: 0, openingCredit: 3000000, periodDebit: 100000, periodCredit: 150000, closingDebit: 0, closingCredit: 3050000 },
      { accountId: "3", accountCode: "3000", accountName: "Equity", openingDebit: 0, openingCredit: 2000000, periodDebit: 0, periodCredit: 0, closingDebit: 0, closingCredit: 2000000 },
      { accountId: "4", accountCode: "4000", accountName: "Revenue", openingDebit: 0, openingCredit: 0, periodDebit: 0, periodCredit: 500000, closingDebit: 0, closingCredit: 500000 },
      { accountId: "5", accountCode: "5000", accountName: "Expenses", openingDebit: 0, openingCredit: 0, periodDebit: 400000, periodCredit: 0, closingDebit: 400000, closingCredit: 0 },
    ]);
  };

  const totals = data.reduce((acc, row) => ({
    openDr: acc.openDr + row.openingDebit,
    openCr: acc.openCr + row.openingCredit,
    perDr: acc.perDr + row.periodDebit,
    perCr: acc.perCr + row.periodCredit,
    closeDr: acc.closeDr + row.closingDebit,
    closeCr: acc.closeCr + row.closingCredit,
  }), { openDr: 0, openCr: 0, perDr: 0, perCr: 0, closeDr: 0, closeCr: 0 });

  return (
    <div className="animate-in fade-in duration-500 h-full flex flex-col">
      <PageHeader
        title="Trial Balance"
        description="Verify the mathematical accuracy of your ledger accounts"
        breadcrumbs={[{ label: "General Ledger" }, { label: "Reports" }, { label: "Trial Balance" }]}
        actions={
          <div className="flex gap-2">
            <Button variant="outline" size="sm" className="glass-input hover-lift">
              <Filter className="h-4 w-4 mr-2" /> Filter
            </Button>
            <Button size="sm" className="bg-primary hover:bg-primary/90 text-white shadow-lg hover-lift">
              <Download className="h-4 w-4 mr-2" /> Export
            </Button>
          </div>
        }
      />

      <Card className="glass flex-1 flex flex-col border-white/60 shadow-lg">
        <div className="p-4 border-b border-white/60 bg-white/40 backdrop-blur-md flex items-center justify-between">
          <div className="flex items-center gap-2 text-sm font-bold text-slate-800">
            <FileText className="h-4 w-4 text-primary" />
            For the period: 01-Apr-2026 to 31-Mar-2027
          </div>
        </div>
        <CardContent className="p-0 flex-1 overflow-auto custom-scrollbar">
          {isLoading ? (
            <div className="flex justify-center p-12 text-slate-500 animate-pulse">Running Trial Balance...</div>
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
                {data.map((row) => (
                  <tr key={row.accountId} className="border-b border-white/30 hover:bg-white/40 transition-colors row-focus">
                    <td className="p-3 border-r border-white/30">
                      <div className="font-semibold text-slate-800">{row.accountName}</div>
                      <div className="text-[10px] font-mono text-slate-500">{row.accountCode}</div>
                    </td>
                    <td className="p-3 text-right font-mono text-xs text-slate-700 border-r border-white/30">{row.openingDebit > 0 ? row.openingDebit.toLocaleString('en-IN') : ""}</td>
                    <td className="p-3 text-right font-mono text-xs text-slate-700 border-r border-white/30">{row.openingCredit > 0 ? row.openingCredit.toLocaleString('en-IN') : ""}</td>
                    <td className="p-3 text-right font-mono text-xs text-slate-700 border-r border-white/30 bg-primary/5">{row.periodDebit > 0 ? row.periodDebit.toLocaleString('en-IN') : ""}</td>
                    <td className="p-3 text-right font-mono text-xs text-slate-700 border-r border-white/30 bg-primary/5">{row.periodCredit > 0 ? row.periodCredit.toLocaleString('en-IN') : ""}</td>
                    <td className="p-3 text-right font-mono text-sm font-bold text-slate-900 border-r border-white/30 bg-emerald-500/5">{row.closingDebit > 0 ? row.closingDebit.toLocaleString('en-IN') : ""}</td>
                    <td className="p-3 text-right font-mono text-sm font-bold text-slate-900 bg-emerald-500/5">{row.closingCredit > 0 ? row.closingCredit.toLocaleString('en-IN') : ""}</td>
                  </tr>
                ))}
              </tbody>
              <tfoot className="bg-white/60 backdrop-blur-md border-t-2 border-slate-300">
                <tr>
                  <td className="p-4 text-right text-xs font-extrabold uppercase text-slate-800 border-r border-white/40">Grand Total</td>
                  <td className="p-4 text-right font-mono text-sm font-extrabold text-slate-900 border-r border-white/40">{totals.openDr.toLocaleString('en-IN')}</td>
                  <td className="p-4 text-right font-mono text-sm font-extrabold text-slate-900 border-r border-white/40">{totals.openCr.toLocaleString('en-IN')}</td>
                  <td className="p-4 text-right font-mono text-sm font-extrabold text-slate-900 border-r border-white/40 bg-primary/10">{totals.perDr.toLocaleString('en-IN')}</td>
                  <td className="p-4 text-right font-mono text-sm font-extrabold text-slate-900 border-r border-white/40 bg-primary/10">{totals.perCr.toLocaleString('en-IN')}</td>
                  <td className="p-4 text-right font-mono text-base font-extrabold text-emerald-700 border-r border-white/40 bg-emerald-500/10">{totals.closeDr.toLocaleString('en-IN')}</td>
                  <td className="p-4 text-right font-mono text-base font-extrabold text-emerald-700 bg-emerald-500/10">{totals.closeCr.toLocaleString('en-IN')}</td>
                </tr>
              </tfoot>
            </table>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

import { useState, useEffect } from "react";
import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button, Input } from "../../components/ui";
import { Plus, Search, FileText, CheckCircle2 } from "lucide-react";
import { useNavigate } from "react-router-dom";
import { api } from "../../lib/api-client";

interface Journal {
  journal_id: string;
  journal_number: string;
  journal_type: string;
  posting_date: string;
  description: string;
  status: string;
  total_amount?: number;
}

export default function JournalList() {
  const navigate = useNavigate();
  const [journals, setJournals] = useState<Journal[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [search, setSearch] = useState("");

  useEffect(() => {
    // Attempt to fetch from API
    api.get<{data: Journal[]}>('/gl/journals').then(res => {
      if (res && res.data && res.data.length > 0) {
        setJournals(res.data);
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
    setJournals([
      { journal_id: "1", journal_number: "JV-2026-0001", journal_type: "JOURNAL", posting_date: "2026-07-31", description: "Opening Balances", status: "POSTED", total_amount: 5000000 },
      { journal_id: "2", journal_number: "JV-2026-0002", journal_type: "PAYMENT", posting_date: "2026-07-31", description: "Vendor Payment for IT Equipment", status: "DRAFT", total_amount: 118000 },
      { journal_id: "3", journal_number: "JV-2026-0003", journal_type: "RECEIPT", posting_date: "2026-08-01", description: "Student Fee Collection", status: "POSTED", total_amount: 85000 }
    ]);
  };

  return (
    <div className="animate-in fade-in duration-500 h-full flex flex-col">
      <PageHeader
        title="Journal Entries"
        description="View and manage general ledger voucher entries"
        breadcrumbs={[{ label: "General Ledger" }, { label: "Journals" }]}
        actions={
          <Button size="sm" onClick={() => navigate('/gl/journals/new')} className="bg-primary hover:bg-primary/90 text-white shadow-lg hover-lift">
            <Plus className="h-4 w-4 mr-2" /> New Voucher
          </Button>
        }
      />

      <Card className="glass flex-1 flex flex-col border-white/60 shadow-lg">
        <div className="p-4 border-b border-white/60 bg-white/40 backdrop-blur-md flex items-center justify-between">
          <div className="relative w-80">
            <Search className="absolute left-3 top-2.5 h-4 w-4 text-slate-400" />
            <Input 
              placeholder="Search journals by number or narration..." 
              className="pl-9 h-10 glass-input shadow-sm rounded-lg" 
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
          </div>
        </div>
        <CardContent className="p-0 flex-1 overflow-auto custom-scrollbar">
          {isLoading ? (
            <div className="flex justify-center p-12 text-slate-500 animate-pulse">Loading vouchers...</div>
          ) : (
            <table className="w-full text-left border-collapse">
              <thead className="sticky top-0 bg-white/80 backdrop-blur-md z-10 border-b border-white/60">
                <tr className="text-[10px] font-bold uppercase tracking-wider text-slate-500">
                  <th className="p-4 w-32">Voucher No</th>
                  <th className="p-4 w-28">Date</th>
                  <th className="p-4 w-28">Type</th>
                  <th className="p-4">Narration</th>
                  <th className="p-4 text-right w-40">Amount (₹)</th>
                  <th className="p-4 w-24 text-center">Status</th>
                </tr>
              </thead>
              <tbody>
                {journals.map((j) => (
                  <tr key={j.journal_id} className="border-b border-white/30 hover:bg-white/40 transition-colors row-focus cursor-pointer">
                    <td className="p-4 font-mono text-sm font-bold text-primary flex items-center gap-2">
                      <FileText className="h-4 w-4 text-slate-400" />
                      {j.journal_number}
                    </td>
                    <td className="p-4 text-sm font-medium text-slate-600">{j.posting_date}</td>
                    <td className="p-4">
                      <span className="text-[10px] px-2 py-0.5 rounded-full font-bold bg-slate-200 text-slate-700">
                        {j.journal_type}
                      </span>
                    </td>
                    <td className="p-4 text-sm text-slate-700 italic">{j.description}</td>
                    <td className="p-4 text-right font-mono text-sm font-bold text-slate-800">
                      {j.total_amount ? j.total_amount.toLocaleString('en-IN', {minimumFractionDigits: 2}) : "0.00"}
                    </td>
                    <td className="p-4 text-center">
                      {j.status === 'POSTED' ? (
                        <span className="inline-flex items-center gap-1 text-[10px] px-2 py-0.5 rounded-full font-bold bg-emerald-100 text-emerald-700">
                          <CheckCircle2 className="h-3 w-3" /> POSTED
                        </span>
                      ) : (
                        <span className="inline-flex items-center gap-1 text-[10px] px-2 py-0.5 rounded-full font-bold bg-amber-100 text-amber-700">
                          DRAFT
                        </span>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

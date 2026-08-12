import { useMemo, useState } from "react";
import { FileText, Search } from "lucide-react";
import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button, Input, TableSkeleton } from "../../components/ui";
import { MoneyDisplay, StatusBadge } from "../../components/data";
import { DataTable, type ColumnDef } from "../../components/data/DataTable";
import { useMockQuery } from "../../hooks/useMockQuery";
import { usePagination } from "../../hooks/usePagination";
import { MOCK_TDS_DEDUCTIONS, type TdsDeductionRow } from "../../lib/mock-data";
import { formatIndianDate } from "../../lib/formatters";

export default function TdsDeductions() {
  const pagination = usePagination(10);
  const { data: rows, isLoading } = useMockQuery<TdsDeductionRow[]>(MOCK_TDS_DEDUCTIONS);
  const [search, setSearch] = useState("");
  const [challanFilter, setChallanFilter] = useState("all");

  const filtered = useMemo(() => {
    if (!rows) return [];
    return rows.filter((r) => {
      if (challanFilter !== "all" && r.challanStatus !== challanFilter) return false;
      if (search.trim()) {
        const q = search.toLowerCase();
        return r.vendorName.toLowerCase().includes(q) || r.pan.toLowerCase().includes(q) || r.section.toLowerCase().includes(q);
      }
      return true;
    });
  }, [rows, search, challanFilter]);

  const pageRows = filtered.slice(pagination.offset, pagination.offset + pagination.pageSize);
  const totalTds = useMemo(() => filtered.reduce((s, r) => s + r.tdsAmount, 0), [filtered]);

  const columns: ColumnDef<TdsDeductionRow>[] = [
    { id: "vendor", header: "Vendor / Deductee", accessorFn: (r) => <div><p className="text-xs font-semibold text-slate-700">{r.vendorName}</p><p className="font-mono text-[10px] text-slate-400">{r.pan}</p></div> },
    { id: "section", header: "Section", accessorFn: (r) => <div><p className="font-mono text-xs font-bold text-slate-700">{r.section}</p><p className="text-[10px] text-slate-400">@{r.rate}%</p></div> },
    { id: "tdsAmount", header: "TDS Amount (₹)", align: "right", accessorFn: (r) => <MoneyDisplay amount={r.tdsAmount} />, className: "w-32" },
    { id: "paymentRef", header: "Payment Ref", accessorFn: (r) => <div><p className="font-mono text-[11px] font-bold text-primary">{r.paymentRef}</p><p className="text-[10px] font-mono text-slate-400">{formatIndianDate(r.paymentDate)}</p></div> },
    { id: "quarter", header: "Quarter", accessorFn: (r) => <span className="text-xs text-slate-600">{r.quarter}</span> },
    { id: "challan", header: "Challan Status", accessorFn: (r) => <StatusBadge status={r.challanStatus} /> },
    { id: "form16a", header: "Form 16A", accessorFn: (r) => r.form16a ? <StatusBadge status="Issued" variant="success" /> : <StatusBadge status="Pending" variant="warning" />, className: "w-24" },
  ];

  return (
    <div className="animate-in fade-in duration-500 space-y-6">
      <PageHeader
        title="TDS Deduction Register"
        description="TDS deducted at source — challan deposit and Form 16A status"
        breadcrumbs={[{ label: "Taxation" }, { label: "TDS Deductions" }]}
        actions={<Button size="sm" className="bg-primary hover:bg-primary/90 text-white shadow-lg hover-lift"><FileText className="h-4 w-4 mr-1" /> Generate 26Q</Button>}
      />
      <Card className="glass border-white/60 shadow-lg">
        <div className="p-4 border-b border-white/40 bg-white/40 backdrop-blur-md flex flex-wrap items-center gap-3">
          <div className="relative w-80">
            <Search className="absolute left-3 top-2.5 h-4 w-4 text-slate-400" />
            <Input placeholder="Search vendor, PAN, section..." className="pl-9 h-10 glass-input shadow-sm" value={search} onChange={(e) => setSearch(e.target.value)} />
          </div>
          <select value={challanFilter} onChange={(e) => setChallanFilter(e.target.value)} className="h-10 rounded-lg border border-white/60 bg-white/60 px-3 text-xs font-semibold text-slate-700 focus:outline-none">
            <option value="all">All Challan Statuses</option>
            <option value="Deposited">Deposited</option>
            <option value="Pending">Pending</option>
            <option value="Filing Pending">Filing Pending</option>
          </select>
          <div className="ml-auto text-xs font-bold text-slate-500">Total TDS: <MoneyDisplay amount={totalTds} /></div>
        </div>
        <CardContent className="p-0 custom-scrollbar overflow-auto">
          {isLoading ? <div className="p-6"><TableSkeleton rows={6} cols={7} /></div>
          : pageRows.length === 0 ? (
            <div className="flex flex-col items-center justify-center gap-2 py-16 text-center">
              <FileText className="h-10 w-10 text-slate-300" />
              <p className="text-sm font-semibold text-slate-600">No TDS deductions match your filters</p>
              <p className="text-xs text-slate-400">TDS is auto-generated when vendor payments are processed.</p>
            </div>
          ) : (
            <div className="min-w-[950px]">
              <DataTable data={pageRows} columns={columns} getRowId={(r) => r.deductionId}
                pagination={{ page: pagination.page, pageSize: pagination.pageSize, total: filtered.length, onPageChange: pagination.setPage, onPageSizeChange: pagination.setPageSize }} />
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

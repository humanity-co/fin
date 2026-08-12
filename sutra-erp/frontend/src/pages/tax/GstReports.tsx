import { useMemo, useState } from "react";
import { Download, FileSpreadsheet } from "lucide-react";
import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button, TableSkeleton } from "../../components/ui";
import { MoneyDisplay, StatusBadge } from "../../components/data";
import { DataTable, type ColumnDef } from "../../components/data/DataTable";
import { useMockQuery } from "../../hooks/useMockQuery";
import { usePagination } from "../../hooks/usePagination";
import { MOCK_GST_RETURNS, type GstReturnPeriod } from "../../lib/mock-data";
import { formatIndianDate } from "../../lib/formatters";

export default function GstReports() {
  const pagination = usePagination(10);
  const { data: returns, isLoading } = useMockQuery<GstReturnPeriod[]>(MOCK_GST_RETURNS);
  const [typeFilter, setTypeFilter] = useState("all");

  const filtered = useMemo(() => {
    if (!returns) return [];
    return returns.filter((r) => typeFilter === "all" || r.filing === typeFilter);
  }, [returns, typeFilter]);

  const pageRows = filtered.slice(pagination.offset, pagination.offset + pagination.pageSize);

  const netPayable = useMemo(() => filtered.filter((r) => r.status !== "Filed").reduce((s, r) => s + r.netPayable, 0), [filtered]);

  const columns: ColumnDef<GstReturnPeriod>[] = [
    { id: "period", header: "Period", accessorFn: (r) => <span className="font-semibold text-slate-800">{r.period}</span> },
    { id: "filing", header: "Form", accessorFn: (r) => <StatusBadge status={r.filing} variant="info" /> },
    { id: "dueDate", header: "Due Date", accessorFn: (r) => <span className="font-mono text-xs text-slate-600">{formatIndianDate(r.dueDate)}</span> },
    { id: "status", header: "Status", accessorFn: (r) => <StatusBadge status={r.status} /> },
    { id: "outward", header: "Outward Taxable (₹)", align: "right", accessorFn: (r) => <MoneyDisplay amount={r.outwardTaxable} />, className: "w-36" },
    { id: "outwardTax", header: "Output Tax (₹)", align: "right", accessorFn: (r) => <MoneyDisplay amount={r.outwardTax} />, className: "w-32" },
    { id: "itc", header: "ITC Availed (₹)", align: "right", accessorFn: (r) => <MoneyDisplay amount={r.inwardItc} />, className: "w-32" },
    { id: "net", header: "Net Payable (₹)", align: "right", accessorFn: (r) => <span className="font-bold text-slate-800"><MoneyDisplay amount={r.netPayable} /></span>, className: "w-32" },
    { id: "filedOn", header: "Filed On", accessorFn: (r) => r.filedOn ? <span className="font-mono text-[11px] text-slate-500">{formatIndianDate(r.filedOn)}</span> : <span className="text-slate-300">—</span> },
  ];

  return (
    <div className="animate-in fade-in duration-500 space-y-6">
      <PageHeader
        title="GST Returns"
        description="GSTR-1 / GSTR-3B / ITC filing status and tax liability by period"
        breadcrumbs={[{ label: "Taxation" }, { label: "GST Returns" }]}
        actions={<Button variant="outline" size="sm" className="glass-input hover-lift"><Download className="h-4 w-4 mr-2" /> Export GSTR-3B</Button>}
      />
      <div className="flex flex-wrap items-center gap-3">
        <select value={typeFilter} onChange={(e) => setTypeFilter(e.target.value)} className="h-9 rounded-lg border border-white/60 bg-white/60 px-3 text-xs font-semibold text-slate-700 focus:outline-none">
          <option value="all">All Forms</option>
          <option value="GSTR-1">GSTR-1</option>
          <option value="GSTR-3B">GSTR-3B</option>
          <option value="GSTR-9">GSTR-9</option>
          <option value="ITC">ITC</option>
        </select>
        <div className="ml-auto rounded-full border border-amber-200 bg-amber-50 px-4 py-1.5 text-xs font-bold text-amber-700">
          Outstanding liability (open periods): <MoneyDisplay amount={netPayable} />
        </div>
      </div>
      <Card className="glass border-white/60 shadow-lg">
        <CardContent className="p-0 custom-scrollbar overflow-auto">
          {isLoading ? <div className="p-6"><TableSkeleton rows={6} cols={9} /></div>
          : pageRows.length === 0 ? (
            <div className="flex flex-col items-center justify-center gap-2 py-16 text-center">
              <FileSpreadsheet className="h-10 w-10 text-slate-300" />
              <p className="text-sm font-semibold text-slate-600">No GST returns for this form</p>
              <p className="text-xs text-slate-400">Returns will appear once invoices are posted for the period.</p>
            </div>
          ) : (
            <div className="min-w-[1050px]">
              <DataTable data={pageRows} columns={columns} getRowId={(r) => `${r.period}-${r.filing}`}
                pagination={{ page: pagination.page, pageSize: pagination.pageSize, total: filtered.length, onPageChange: pagination.setPage, onPageSizeChange: pagination.setPageSize }} />
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

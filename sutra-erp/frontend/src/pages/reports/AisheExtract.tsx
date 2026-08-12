import { useMemo, useState } from "react";
import { Download, GraduationCap, Search } from "lucide-react";
import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button, Input, TableSkeleton } from "../../components/ui";
import { DataTable, type ColumnDef } from "../../components/data/DataTable";
import { useMockQuery } from "../../hooks/useMockQuery";
import { usePagination } from "../../hooks/usePagination";
import { MOCK_AISHE, type AisheProgram } from "../../lib/mock-data";
import { formatIndianNumber } from "../../lib/formatters";

export default function AisheExtract() {
  const pagination = usePagination(10);
  const { data: programs, isLoading } = useMockQuery<AisheProgram[]>(MOCK_AISHE);
  const [search, setSearch] = useState("");
  const [levelFilter, setLevelFilter] = useState("all");

  const filtered = useMemo(() => {
    if (!programs) return [];
    return programs.filter((p) => {
      if (levelFilter !== "all" && p.level !== levelFilter) return false;
      if (search.trim()) return p.program.toLowerCase().includes(search.toLowerCase());
      return true;
    });
  }, [programs, search, levelFilter]);

  const pageRows = filtered.slice(pagination.offset, pagination.offset + pagination.pageSize);

  const totals = useMemo(() => {
    const list = filtered;
    return { students: list.reduce((s, p) => s + p.admitted, 0), female: list.reduce((s, p) => s + p.female, 0), faculty: list.reduce((s, p) => s + p.faculty, 0) };
  }, [filtered]);

  const columns: ColumnDef<AisheProgram>[] = [
    { id: "program", header: "Program", accessorFn: (r) => <span className="font-semibold text-slate-800">{r.program}</span> },
    { id: "level", header: "Level", accessorFn: (r) => <span className="text-xs text-slate-500">{r.level}</span> },
    { id: "intake", header: "Intake", align: "right", accessorFn: (r) => <span className="font-mono">{r.intake}</span>, className: "w-20" },
    { id: "admitted", header: "Admitted", align: "right", accessorFn: (r) => <span className="font-mono font-bold">{r.admitted}</span>, className: "w-24" },
    { id: "male", header: "Male", align: "right", accessorFn: (r) => <span className="font-mono">{r.male}</span>, className: "w-20" },
    { id: "female", header: "Female", align: "right", accessorFn: (r) => <span className="font-mono">{r.female}</span>, className: "w-20" },
    { id: "faculty", header: "Faculty", align: "right", accessorFn: (r) => <span className="font-mono">{r.faculty}</span>, className: "w-20" },
    { id: "phd", header: "PhD", align: "right", accessorFn: (r) => <span className="font-mono">{r.phd}</span>, className: "w-20" },
    { id: "minority", header: "Minority", align: "right", accessorFn: (r) => <span className="font-mono">{r.minority}</span>, className: "w-20" },
  ];

  return (
    <div className="animate-in fade-in duration-500 space-y-6">
      <PageHeader
        title="AISHE Extract"
        description="All India Survey on Higher Education — program-wise enrolment data (AY 2026-27)"
        breadcrumbs={[{ label: "Reports" }, { label: "AISHE Extract" }]}
        actions={<Button size="sm" className="bg-primary hover:bg-primary/90 text-white shadow-lg hover-lift"><Download className="h-4 w-4 mr-1" /> Download AISHE CSV</Button>}
      />
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Kpi label="Total Students" value={formatIndianNumber(totals.students)} />
        <Kpi label="Female Students" value={formatIndianNumber(totals.female)} />
        <Kpi label="Teaching Faculty" value={formatIndianNumber(totals.faculty)} />
      </div>
      <Card className="glass border-white/60 shadow-lg">
        <div className="p-4 border-b border-white/40 bg-white/40 backdrop-blur-md flex flex-wrap items-center gap-3">
          <div className="relative w-80">
            <Search className="absolute left-3 top-2.5 h-4 w-4 text-slate-400" />
            <Input placeholder="Search program..." className="pl-9 h-10 glass-input shadow-sm" value={search} onChange={(e) => setSearch(e.target.value)} />
          </div>
          <select value={levelFilter} onChange={(e) => setLevelFilter(e.target.value)} className="h-10 rounded-lg border border-white/60 bg-white/60 px-3 text-xs font-semibold text-slate-700 focus:outline-none">
            <option value="all">All Levels</option>
            <option value="UG">UG</option><option value="PG">PG</option><option value="Research">Research</option><option value="Diploma">Diploma</option>
          </select>
        </div>
        <CardContent className="p-0 custom-scrollbar overflow-auto">
          {isLoading ? <div className="p-6"><TableSkeleton rows={7} cols={9} /></div>
          : pageRows.length === 0 ? (
            <div className="flex flex-col items-center justify-center gap-2 py-16 text-center">
              <GraduationCap className="h-10 w-10 text-slate-300" />
              <p className="text-sm font-semibold text-slate-600">No programs match your filters</p>
              <p className="text-xs text-slate-400">Program data will populate once enrolment is finalised.</p>
            </div>
          ) : (
            <div className="min-w-[900px]">
              <DataTable data={pageRows} columns={columns} getRowId={(r) => r.program}
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

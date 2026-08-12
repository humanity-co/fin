import { useMemo, useState } from "react";
import { Plus, Search, ShieldCheck, Users } from "lucide-react";
import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button, Input, TableSkeleton, Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "../../components/ui";
import { StatusBadge } from "../../components/data";
import { DataTable, type ColumnDef } from "../../components/data/DataTable";
import { useMockQuery } from "../../hooks/useMockQuery";
import { usePagination } from "../../hooks/usePagination";
import { MOCK_USERS, type UserMock } from "../../lib/mock-data";

const ROLES = ["CFO", "Registrar", "Accountant", "Auditor", "HOD", "Principal"];

export default function UserManagement() {
  const pagination = usePagination(10);
  const { data: users, isLoading } = useMockQuery<UserMock[]>(MOCK_USERS);
  const [search, setSearch] = useState("");
  const [roleFilter, setRoleFilter] = useState("all");
  const [inviteOpen, setInviteOpen] = useState(false);
  const [invited, setInvited] = useState(false);

  const filtered = useMemo(() => {
    if (!users) return [];
    return users.filter((u) => {
      if (roleFilter !== "all" && u.role !== roleFilter) return false;
      if (search.trim()) {
        const q = search.toLowerCase();
        return u.name.toLowerCase().includes(q) || u.email.toLowerCase().includes(q) || (u.department ?? "").toLowerCase().includes(q);
      }
      return true;
    });
  }, [users, search, roleFilter]);

  const pageRows = filtered.slice(pagination.offset, pagination.offset + pagination.pageSize);

  const columns: ColumnDef<UserMock>[] = [
    { id: "name", header: "User", accessorFn: (r) => <div><p className="font-semibold text-slate-800">{r.name}</p><p className="text-[10px] text-slate-400">{r.email}</p></div> },
    { id: "role", header: "Role", accessorFn: (r) => <StatusBadge status={r.role} variant={r.role === "CFO" ? "info" : "secondary"} /> },
    { id: "dept", header: "Department", accessorFn: (r) => <span className="text-xs text-slate-600">{r.department ?? "—"}</span> },
    {
      id: "permissions", header: "Permissions",
      accessorFn: (r) => (
        <div className="flex items-center gap-1.5">
          <ShieldCheck className="h-3.5 w-3.5 text-emerald-600" />
          <span className="font-mono text-xs font-bold text-slate-700">{r.permissions}/41</span>
        </div>
      ),
      className: "w-28",
    },
    { id: "lastLogin", header: "Last Login", accessorFn: (r) => <span className="font-mono text-[11px] text-slate-500">{r.lastLogin}</span>, className: "w-36" },
    { id: "active", header: "Status", accessorFn: (r) => r.isActive ? <StatusBadge status="Active" /> : <StatusBadge status="Inactive" />, className: "w-24" },
  ];

  const handleInvite = () => {
    setInvited(true);
    setTimeout(() => { setInvited(false); setInviteOpen(false); }, 1800);
  };

  return (
    <div className="animate-in fade-in duration-500 space-y-6">
      <PageHeader
        title="User Management"
        description="Users, roles and permission grants (41 permissions across 14 roles)"
        breadcrumbs={[{ label: "System" }, { label: "User Management" }]}
        actions={<Button size="sm" className="bg-primary hover:bg-primary/90 text-white shadow-lg hover-lift" onClick={() => setInviteOpen(true)}><Plus className="h-4 w-4 mr-1" /> Invite User</Button>}
      />
      <Card className="glass border-white/60 shadow-lg">
        <div className="p-4 border-b border-white/40 bg-white/40 backdrop-blur-md flex flex-wrap items-center gap-3">
          <div className="relative w-80">
            <Search className="absolute left-3 top-2.5 h-4 w-4 text-slate-400" />
            <Input placeholder="Search name, email, department..." className="pl-9 h-10 glass-input shadow-sm" value={search} onChange={(e) => setSearch(e.target.value)} />
          </div>
          <select value={roleFilter} onChange={(e) => setRoleFilter(e.target.value)} className="h-10 rounded-lg border border-white/60 bg-white/60 px-3 text-xs font-semibold text-slate-700 focus:outline-none">
            <option value="all">All Roles</option>
            {ROLES.map((r) => <option key={r} value={r}>{r}</option>)}
          </select>
          <div className="ml-auto text-xs font-bold text-slate-500">{filtered.length} users</div>
        </div>
        <CardContent className="p-0 custom-scrollbar overflow-auto">
          {isLoading ? <div className="p-6"><TableSkeleton rows={6} cols={6} /></div>
          : pageRows.length === 0 ? (
            <div className="flex flex-col items-center justify-center gap-2 py-16 text-center">
              <Users className="h-10 w-10 text-slate-300" />
              <p className="text-sm font-semibold text-slate-600">No users match your filters</p>
              <p className="text-xs text-slate-400">Invite a user to grant role-based module access.</p>
            </div>
          ) : (
            <div className="min-w-[850px]">
              <DataTable data={pageRows} columns={columns} getRowId={(r) => r.userId}
                pagination={{ page: pagination.page, pageSize: pagination.pageSize, total: filtered.length, onPageChange: pagination.setPage, onPageSizeChange: pagination.setPageSize }} />
            </div>
          )}
        </CardContent>
      </Card>

      <Dialog open={inviteOpen} onOpenChange={setInviteOpen}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle className="text-lg">Invite User</DialogTitle>
            <DialogDescription>An email invite with role-based access will be sent.</DialogDescription>
          </DialogHeader>
          {invited ? (
            <div className="py-8 text-center text-sm font-bold text-emerald-700">Invitation sent successfully ✓</div>
          ) : (
            <div className="space-y-4 text-sm">
              <div>
                <label className="mb-1 block text-[10px] font-bold uppercase tracking-wider text-slate-500">Email</label>
                <Input placeholder="name@srcoe.edu.in" className="glass-input" />
              </div>
              <div>
                <label className="mb-1 block text-[10px] font-bold uppercase tracking-wider text-slate-500">Role</label>
                <select className="h-10 w-full rounded-lg border border-white/60 bg-white/60 px-3 text-sm font-semibold text-slate-700 focus:outline-none">
                  {ROLES.map((r) => <option key={r}>{r}</option>)}
                </select>
              </div>
              <div>
                <label className="mb-1 block text-[10px] font-bold uppercase tracking-wider text-slate-500">Scope</label>
                <select className="h-10 w-full rounded-lg border border-white/60 bg-white/60 px-3 text-sm font-semibold text-slate-700 focus:outline-none">
                  <option>Global</option><option>Campus</option><option>Department</option>
                </select>
              </div>
            </div>
          )}
          <DialogFooter>
            <Button variant="outline" onClick={() => setInviteOpen(false)}>Cancel</Button>
            {!invited && <Button className="bg-primary text-white" onClick={handleInvite}>Send Invite</Button>}
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

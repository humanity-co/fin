import { useMemo, useState } from "react";
import { useNavigate } from "react-router";
import { Plus, Search, Folder, File, ChevronRight, ChevronDown, BookOpen } from "lucide-react";
import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button, Input, TableSkeleton } from "../../components/ui";
import { MoneyDisplay } from "../../components/data";
import { useMockQuery } from "../../hooks/useMockQuery";
import { COA_ACCOUNTS, type Account, type AccountType } from "../../lib/mock-data";

const ACCOUNT_TYPES: AccountType[] = ["Asset", "Liability", "Equity", "Income", "Expense"];

interface TreeNode extends Account {
  level: number;
  children: TreeNode[];
}

/**
 * Chart of Accounts — hierarchical tree/table with expandable groups,
 * search, type filter, balance summary and drill-down to account ledgers.
 */
export default function ChartOfAccounts() {
  const navigate = useNavigate();
  const { data: accounts, isLoading } = useMockQuery<Account[]>(COA_ACCOUNTS);
  const [search, setSearch] = useState("");
  const [typeFilter, setTypeFilter] = useState<string>("all");
  const [expanded, setExpanded] = useState<Set<string>>(() => new Set(["acc-1000", "acc-2000", "acc-3000", "acc-4000", "acc-5000"]));

  const visible = useMemo(() => {
    if (!accounts) return [];
    const q = search.trim().toLowerCase();
    return accounts.filter((a) => {
      if (typeFilter !== "all" && a.account_type !== typeFilter) return false;
      if (!q) return true;
      return (
        a.account_name.toLowerCase().includes(q) ||
        a.account_code.toLowerCase().includes(q)
      );
    });
  }, [accounts, search, typeFilter]);

  const tree = useMemo(() => {
    const build = (parentId: string | null, level: number): TreeNode[] =>
      visible
        .filter((a) => (parentId ? a.parent_account_id === parentId : !a.parent_account_id))
        .map((a) => ({ ...a, level, children: build(a.account_id, level + 1) }));
    return build(null, 0);
  }, [visible]);

  const totals = useMemo(() => {
    const list = accounts ?? [];
    return {
      accounts: list.length,
      debit: list.filter((a) => a.current_balance > 0).reduce((s, a) => s + a.current_balance, 0),
      credit: list.filter((a) => a.current_balance < 0).reduce((s, a) => s + Math.abs(a.current_balance), 0),
    };
  }, [accounts]);

  const toggleExpand = (id: string) =>
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  const renderTree = (nodes: TreeNode[]) => {
    return nodes.map((node) => (
      <div key={node.account_id} className="w-full">
        <div
          className={`flex items-center border-b border-white/30 transition-all ${node.level === 0 ? "bg-white/50 font-bold" : "hover:bg-white/40"} ${node.children.length > 0 ? "cursor-pointer" : ""}`}
          style={{ paddingLeft: `${node.level * 1.5 + 0.75}rem` }}
          onClick={() => node.children.length > 0 && toggleExpand(node.account_id)}
        >
          <div className="flex items-center gap-2 flex-1 py-2.5">
            {node.children.length > 0 ? (
              expanded.has(node.account_id) ? <ChevronDown className="h-4 w-4 text-slate-500" /> : <ChevronRight className="h-4 w-4 text-slate-500" />
            ) : (
              <span className="w-4" />
            )}
            {node.children.length > 0 ? <Folder className="h-4 w-4 text-indigo-500" /> : <File className="h-4 w-4 text-slate-400" />}
            <span className="text-slate-500 font-mono text-xs">{node.account_code}</span>
            {node.children.length > 0 ? (
              <span className="text-slate-800 text-sm">{node.account_name}</span>
            ) : (
              <button
                className="text-sm text-slate-700 hover:text-primary hover:underline cursor-pointer"
                onClick={(e) => {
                  e.stopPropagation();
                  navigate(`/gl/ledger/${node.account_id}`);
                }}
                title="Open account ledger"
              >
                {node.account_name}
              </button>
            )}
            {node.gst_applicable && (
              <span className="text-[9px] px-1.5 py-0.5 rounded bg-amber-100 text-amber-700 font-bold">GST</span>
            )}
            {node.aishe_mapped && (
              <span className="text-[9px] px-1.5 py-0.5 rounded bg-sky-100 text-sky-700 font-bold">AISHE</span>
            )}
            {node.naac_mapped && (
              <span className="text-[9px] px-1.5 py-0.5 rounded bg-violet-100 text-violet-700 font-bold">NAAC</span>
            )}
          </div>
          <div className="w-40 text-right pr-3">
            <MoneyDisplay amount={node.current_balance} variant="accounting" className="text-sm font-semibold" />
          </div>
          <div className="w-24 text-right pr-3">
            <span className={`text-[10px] px-2 py-0.5 rounded-full font-semibold ${typeColor(node.account_type)}`}>
              {node.account_type}
            </span>
          </div>
        </div>
        {node.children.length > 0 && expanded.has(node.account_id) && <div className="w-full">{renderTree(node.children)}</div>}
      </div>
    ));
  };

  return (
    <div className="animate-in fade-in duration-500 space-y-6">
      <PageHeader
        title="Chart of Accounts"
        description="Manage the general ledger account hierarchy and opening balances"
        breadcrumbs={[{ label: "General Ledger" }, { label: "Chart of Accounts" }]}
        actions={
          <Button size="sm" className="bg-primary hover:bg-primary/90 text-white shadow-lg hover-lift">
            <Plus className="h-4 w-4 mr-1" /> New Account
          </Button>
        }
      />

      {/* Summary strip */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Card className="glass border-white/60 p-4 shadow-sm">
          <p className="text-[10px] font-bold uppercase tracking-widest text-slate-500">Accounts</p>
          <p className="mt-1 text-xl font-extrabold text-slate-800">{totals.accounts}</p>
          <p className="text-[10px] text-slate-400 font-medium">5-level hierarchy · all entities</p>
        </Card>
        <Card className="glass border-white/60 p-4 shadow-sm">
          <p className="text-[10px] font-bold uppercase tracking-widest text-slate-500">Total Debit Balances</p>
          <p className="mt-1 text-xl font-extrabold text-slate-800"><MoneyDisplay amount={totals.debit} /></p>
          <p className="text-[10px] text-slate-400 font-medium">Assets & Expenses</p>
        </Card>
        <Card className="glass border-white/60 p-4 shadow-sm">
          <p className="text-[10px] font-bold uppercase tracking-widest text-slate-500">Total Credit Balances</p>
          <p className="mt-1 text-xl font-extrabold text-emerald-700"><MoneyDisplay amount={totals.credit} /></p>
          <p className="text-[10px] text-slate-400 font-medium">Liabilities, Funds & Income</p>
        </Card>
      </div>

      <Card className="glass border-white/60 shadow-lg">
        <div className="p-4 border-b border-white/40 bg-white/40 backdrop-blur-md flex flex-wrap items-center gap-3">
          <div className="relative w-80">
            <Search className="absolute left-3 top-2.5 h-4 w-4 text-slate-400" />
            <Input
              placeholder="Search accounts by code or name..."
              className="pl-9 h-10 glass-input shadow-sm"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
          </div>
          <select
            value={typeFilter}
            onChange={(e) => setTypeFilter(e.target.value)}
            className="h-10 rounded-lg border border-white/60 bg-white/60 px-3 text-xs font-semibold text-slate-700 shadow-sm focus:outline-none focus:ring-2 focus:ring-primary/40"
          >
            <option value="all">All Account Types</option>
            {ACCOUNT_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
          </select>
          <div className="ml-auto flex gap-2">
            <Button variant="outline" size="sm" onClick={() => setExpanded(new Set((visible ?? []).map((a) => a.account_id)))}>
              Expand All
            </Button>
            <Button variant="outline" size="sm" onClick={() => setExpanded(new Set())}>Collapse All</Button>
          </div>
        </div>
        <CardContent className="p-0 custom-scrollbar overflow-auto max-h-[640px]">
          {isLoading ? (
            <div className="p-6"><TableSkeleton rows={10} cols={4} /></div>
          ) : tree.length === 0 ? (
            <div className="flex flex-col items-center justify-center gap-2 py-16 text-center">
              <BookOpen className="h-10 w-10 text-slate-300" />
              <p className="text-sm font-semibold text-slate-600">No accounts found</p>
              <p className="text-xs text-slate-400">Try a different search term or account type.</p>
            </div>
          ) : (
            <div className="min-w-[800px]">
              <div className="flex items-center border-b border-white/40 bg-slate-50/60 font-semibold text-xs tracking-wider text-slate-500 uppercase px-3 py-2.5">
                <div className="flex-1 pl-8">Account Details</div>
                <div className="w-40 text-right pr-3">Current Balance</div>
                <div className="w-24 text-right pr-3">Type</div>
              </div>
              {renderTree(tree)}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function typeColor(type: string): string {
  switch (type) {
    case "Asset": return "bg-emerald-100 text-emerald-700";
    case "Liability": return "bg-rose-100 text-rose-700";
    case "Equity": return "bg-violet-100 text-violet-700";
    case "Income": return "bg-sky-100 text-sky-700";
    case "Expense": return "bg-amber-100 text-amber-700";
    default: return "bg-slate-100 text-slate-600";
  }
}

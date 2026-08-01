import { useState, useEffect } from "react";
import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button, Input } from "../../components/ui";
import { Plus, Search, Folder, File, ChevronRight, ChevronDown } from "lucide-react";
import { api } from "../../lib/api-client";

interface Account {
  account_id: string;
  account_code: string;
  account_name: string;
  account_type: string;
  level: number;
  parent_account_id?: string;
  opening_balance: number;
  current_balance: number;
  is_active: boolean;
  children?: Account[];
}

export default function ChartOfAccounts() {
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [search, setSearch] = useState("");
  const [expanded, setExpanded] = useState<Set<string>>(new Set());

  useEffect(() => {
    fetchAccounts();
  }, []);

  const fetchAccounts = async () => {
    setIsLoading(true);
    try {
      // Temporary fallback data if API returns empty
      const res = await api.get<{data: Account[]}>('/gl/accounts');
      if (res && res.data.length > 0) {
        setAccounts(res.data);
      } else {
        // Provide mock initial structure if empty
        setAccounts([
          { account_id: "1", account_code: "1000", account_name: "Assets", account_type: "ASSET", level: 1, opening_balance: 0, current_balance: 5000000, is_active: true },
          { account_id: "11", account_code: "1100", account_name: "Current Assets", account_type: "ASSET", level: 2, parent_account_id: "1", opening_balance: 0, current_balance: 5000000, is_active: true },
          { account_id: "111", account_code: "1110", account_name: "Bank Accounts", account_type: "ASSET", level: 3, parent_account_id: "11", opening_balance: 0, current_balance: 5000000, is_active: true },
          { account_id: "2", account_code: "2000", account_name: "Liabilities", account_type: "LIABILITY", level: 1, opening_balance: 0, current_balance: 0, is_active: true },
          { account_id: "3", account_code: "3000", account_name: "Equity", account_type: "EQUITY", level: 1, opening_balance: 0, current_balance: 0, is_active: true },
          { account_id: "4", account_code: "4000", account_name: "Revenue", account_type: "INCOME", level: 1, opening_balance: 0, current_balance: 0, is_active: true },
          { account_id: "5", account_code: "5000", account_name: "Expenses", account_type: "EXPENSE", level: 1, opening_balance: 0, current_balance: 0, is_active: true },
        ]);
      }
    } catch (e) {
      console.error(e);
    } finally {
      setIsLoading(false);
    }
  };

  const toggleExpand = (id: string) => {
    const newExpanded = new Set(expanded);
    if (newExpanded.has(id)) newExpanded.delete(id);
    else newExpanded.add(id);
    setExpanded(newExpanded);
  };

  // Build tree
  const buildTree = (parentId?: string): any[] => {
    return accounts
      .filter(a => parentId ? a.parent_account_id === parentId : !a.parent_account_id)
      .map(account => {
        const children = buildTree(account.account_id);
        return { ...account, children };
      });
  };

  const renderTree = (nodes: any[]) => {
    return nodes.map(node => (
      <div key={node.account_id} className="w-full">
        <div 
          className={`flex items-center p-3 border-b hover:bg-black/5 dark:hover:bg-white/5 cursor-pointer row-focus transition-all ${node.level === 1 ? 'font-bold bg-muted/20' : ''}`}
          style={{ paddingLeft: `${node.level * 1.5}rem` }}
          onClick={() => toggleExpand(node.account_id)}
        >
          <div className="flex items-center gap-2 flex-1">
            {node.children.length > 0 ? (
              expanded.has(node.account_id) ? <ChevronDown className="h-4 w-4 text-slate-500" /> : <ChevronRight className="h-4 w-4 text-slate-500" />
            ) : (
              <span className="w-4" />
            )}
            {node.children.length > 0 ? <Folder className="h-4 w-4 text-indigo-500" /> : <File className="h-4 w-4 text-slate-400" />}
            <span className="text-slate-500 font-mono text-xs">{node.account_code}</span>
            <span className="text-slate-800">{node.account_name}</span>
          </div>
          <div className="w-32 text-right font-medium text-slate-600">
            {(node.current_balance / 100).toLocaleString('en-IN', { style: 'currency', currency: 'INR' })}
          </div>
          <div className="w-24 text-right">
            <span className={`text-[10px] px-2 py-0.5 rounded-full ${node.is_active ? 'bg-emerald-100 text-emerald-700' : 'bg-rose-100 text-rose-700'}`}>
              {node.account_type}
            </span>
          </div>
        </div>
        {node.children.length > 0 && expanded.has(node.account_id) && (
          <div className="w-full">
            {renderTree(node.children)}
          </div>
        )}
      </div>
    ));
  };

  return (
    <div className="animate-in fade-in duration-500">
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

      <Card>
        <div className="p-4 border-b flex items-center justify-between bg-white/40 backdrop-blur-md">
          <div className="relative w-80">
            <Search className="absolute left-3 top-2.5 h-4 w-4 text-slate-400" />
            <Input 
              placeholder="Search accounts by code or name..." 
              className="pl-9 h-10 glass-input shadow-sm rounded-lg" 
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
          </div>
          <div className="flex gap-2">
            <Button variant="outline" size="sm" onClick={() => setExpanded(new Set(accounts.map(a => a.account_id)))}>Expand All</Button>
            <Button variant="outline" size="sm" onClick={() => setExpanded(new Set())}>Collapse All</Button>
          </div>
        </div>
        <CardContent className="p-0 custom-scrollbar overflow-auto max-h-[600px]">
          {isLoading ? (
            <div className="flex justify-center p-12 text-slate-500 animate-pulse">Loading accounts...</div>
          ) : (
            <div className="min-w-[800px]">
              <div className="flex items-center p-3 border-b bg-slate-50 font-semibold text-xs tracking-wider text-slate-500 uppercase">
                <div className="flex-1 pl-12">Account Details</div>
                <div className="w-32 text-right pr-2">Current Balance</div>
                <div className="w-24 text-right pr-4">Type</div>
              </div>
              {renderTree(buildTree())}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

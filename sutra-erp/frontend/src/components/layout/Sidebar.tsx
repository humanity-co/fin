import { useState } from "react";
import { NavLink, useLocation, useNavigate } from "react-router-dom";
import {
  LayoutDashboard, BookOpen, FileText, PieChart, Users, GraduationCap, Award,
  Undo2, Building2, ListOrdered, FileSearch, Banknote, Landmark, Scale, Percent,
  BarChart3, CalendarClock, Settings, ChevronDown, ChevronRight, Sparkles,
  type LucideIcon,
} from "lucide-react";
import { cn } from "../../lib/utils";
import { useKeyboardShortcut, useShortcutDisplay } from "../../hooks/useKeyboardShortcut";
import { useCurrentRoles } from "../../hooks/useDashboardRole";

interface NavItem {
  label: string;
  href: string;
  icon: LucideIcon;
  shortcut?: string;
  /** Roles allowed to see this item — undefined = visible to all. */
  roles?: string[];
}

interface NavGroup {
  group: string;
  collapsible?: boolean;
  items: NavItem[];
}

const NAVIGATION: NavGroup[] = [
  {
    group: "Gateway",
    items: [
      { label: "Dashboard", href: "/dashboard", icon: LayoutDashboard, shortcut: "Alt+D" },
      { label: "Compliance Calendar", href: "/dashboard/compliance", icon: CalendarClock },
    ],
  },
  {
    group: "General Ledger",
    items: [
      { label: "Chart of Accounts", href: "/gl/accounts", icon: BookOpen, shortcut: "Alt+A" },
      { label: "Journal Vouchers", href: "/gl/journals", icon: FileText, shortcut: "Alt+V" },
      { label: "Trial Balance", href: "/gl/reports/trial-balance", icon: PieChart, shortcut: "Alt+T" },
      { label: "Account Ledger", href: "/gl/ledger/acc-1110", icon: Scale },
    ],
  },
  {
    group: "Accounts Receivable",
    items: [
      { label: "Fee Structures", href: "/ar/fee-structures", icon: GraduationCap },
      { label: "Fee Collection", href: "/ar/payments/receipts", icon: Banknote },
      { label: "Scholarships", href: "/ar/scholarships", icon: Award },
      { label: "Refunds", href: "/ar/refunds", icon: Undo2 },
    ],
  },
  {
    group: "Accounts Payable",
    items: [
      { label: "Vendors", href: "/ap/vendors", icon: Building2 },
      { label: "Purchase Orders", href: "/ap/purchase-orders", icon: ListOrdered },
      { label: "Purchase Invoices", href: "/ap/purchase-invoices", icon: FileSearch },
      { label: "Payments", href: "/ap/payments", icon: Banknote },
    ],
  },
  {
    group: "Treasury",
    items: [
      { label: "Bank Accounts", href: "/treasury/bank-accounts", icon: Landmark },
      { label: "Reconciliation", href: "/treasury/reconciliation", icon: Scale },
    ],
  },
  {
    group: "Taxation",
    items: [
      { label: "GST Returns", href: "/tax/gst/registrations", icon: Percent },
      { label: "TDS Deductions", href: "/tax/tds/deductions", icon: FileText },
    ],
  },
  {
    group: "Reports & Compliance",
    items: [
      { label: "NAAC Dashboard", href: "/reports", icon: BarChart3 },
      { label: "AISHE Extract", href: "/reports/aishe", icon: Users },
      { label: "Compliance Calendar", href: "/dashboard/compliance", icon: CalendarClock },
    ],
  },
  {
    group: "System",
    items: [
      { label: "Settings", href: "/settings", icon: Settings, roles: ["CFO", "Admin"] },
      { label: "User Management", href: "/settings/users", icon: Users, roles: ["CFO", "Admin"] },
    ],
  },
];

/** Role groups that map dashboard roles to module visibility. */
export function Sidebar() {
  const location = useLocation();
  const navigate = useNavigate();
  const { format } = useShortcutDisplay();
  const roles = useCurrentRoles().map((r) => r.toLowerCase());

  // Collapsible group state — expanded by default; auto-expands the active group.
  const [collapsed, setCollapsed] = useState<Set<string>>(() => {
    const active = NAVIGATION.find((g) => g.items.some((i) => location.pathname.startsWith(i.href)));
    return new Set(NAVIGATION.filter((g) => g.group !== active?.group).map((g) => g.group));
  });

  // Register Global Shortcuts
  useKeyboardShortcut('v', () => navigate('/gl/journals'), { alt: true });
  useKeyboardShortcut('d', () => navigate('/dashboard'), { alt: true });
  useKeyboardShortcut('a', () => navigate('/gl/accounts'), { alt: true });
  useKeyboardShortcut('t', () => navigate('/gl/reports/trial-balance'), { alt: true });

  const toggleGroup = (group: string) =>
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(group)) next.delete(group);
      else next.add(group);
      return next;
    });

  const canSee = (item: NavItem) => {
    if (!item.roles || item.roles.length === 0) return true;
    return item.roles.some((r) => roles.includes(r.toLowerCase()));
  };

  return (
    <aside className="flex w-64 flex-col glass-sidebar shadow-2xl relative overflow-hidden z-20">
      {/* Background ambient glow */}
      <div className="absolute top-0 left-0 right-0 h-64 bg-gradient-to-br from-primary/20 to-transparent blur-3xl opacity-50 pointer-events-none" />
      {/* Brand */}
      <div className="flex h-16 items-center gap-3 px-6 relative z-10 border-b border-white/5">
        <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-gradient-to-br from-primary to-indigo-600 font-bold text-white shadow-[0_0_15px_rgba(124,58,237,0.5)]">
          <Sparkles className="h-4 w-4" />
        </div>
        <span className="text-lg font-bold tracking-tight text-white font-['Outfit']">Sutra ERP</span>
      </div>
      {/* Navigation */}
      <nav className="flex-1 overflow-y-auto py-6 custom-scrollbar relative z-10 px-3">
        {NAVIGATION.map((group) => {
          const visibleItems = group.items.filter(canSee);
          if (visibleItems.length === 0) return null;
          const isCollapsed = collapsed.has(group.group);
          const groupActive = visibleItems.some((i) => location.pathname.startsWith(i.href));
          return (
            <div key={group.group} className="mb-3">
              <button
                onClick={() => toggleGroup(group.group)}
                className={cn(
                  "mb-1.5 flex w-full items-center justify-between rounded-lg px-3 py-1.5 text-left transition-colors",
                  groupActive ? "text-primary" : "text-slate-400/80 hover:text-slate-200"
                )}
              >
                <span className="text-[10px] font-bold uppercase tracking-widest">{group.group}</span>
                {isCollapsed ? <ChevronRight className="h-3.5 w-3.5" /> : <ChevronDown className="h-3.5 w-3.5" />}
              </button>
              {!isCollapsed && (
                <ul className="space-y-0.5">
                  {visibleItems.map((item) => {
                    const isActive =
                      location.pathname.startsWith(item.href) &&
                      (item.href !== "/dashboard" || location.pathname === "/dashboard" || location.pathname === "/");
                    return (
                      <li key={item.href}>
                        <NavLink
                          to={item.href}
                          className={cn(
                            "group flex items-center justify-between rounded-lg px-3 py-2 text-sm font-medium transition-all duration-300 relative overflow-hidden",
                            isActive
                              ? "text-white shadow-md bg-white/10 border border-white/10"
                              : "text-slate-300/80 hover:bg-white/5 hover:text-white"
                          )}
                        >
                          {isActive && <div className="absolute left-0 top-0 bottom-0 w-1 bg-primary rounded-r-full" />}
                          <div className="flex items-center gap-3 relative z-10">
                            <item.icon className={cn(
                              "h-4 w-4 transition-transform duration-300",
                              isActive ? "text-primary scale-110" : "text-slate-500 group-hover:text-slate-300"
                            )} />
                            {item.label}
                          </div>
                          {item.shortcut && (
                            <span className={cn(
                              "text-[10px] px-1.5 py-0.5 rounded-md border font-mono tracking-tighter transition-colors",
                              isActive
                                ? "border-primary/50 text-primary-100 bg-primary/20"
                                : "border-slate-700/50 text-slate-500 group-hover:border-slate-500"
                            )}>
                              {format(item.shortcut)}
                            </span>
                          )}
                        </NavLink>
                      </li>
                    );
                  })}
                </ul>
              )}
            </div>
          );
        })}
      </nav>
      {/* Footer */}
      <div className="border-t border-white/5 p-4 relative z-10 bg-black/20 backdrop-blur-md">
        <div className="flex items-center gap-2 text-[11px] font-medium text-slate-300/80">
          <div className="h-2 w-2 rounded-full bg-emerald-400 shadow-[0_0_10px_rgba(52,211,153,0.8)] animate-pulse" />
          FY 2026-27 · Shri Ram College of Engineering
        </div>
      </div>
    </aside>
  );
}

import { NavLink, useLocation, useNavigate } from "react-router-dom";
import { cn } from "../../lib/utils";
import {
  BookOpen,
  LayoutDashboard,
  FileText,
  PieChart,
  Settings,
  ListOrdered,
  Users,
  Building2,
  type LucideIcon,
  Sparkles,
} from "lucide-react";
import { useKeyboardShortcut, useShortcutDisplay } from "../../hooks/useKeyboardShortcut";

interface NavItem {
  label: string;
  href: string;
  icon: LucideIcon;
  shortcut?: string;
}

interface NavGroup {
  group: string;
  items: NavItem[];
}

const navigation: NavGroup[] = [
  {
    group: "Gateway",
    items: [
      { label: "Dashboard", href: "/dashboard", icon: LayoutDashboard, shortcut: "Alt+D" },
    ]
  },
  {
    group: "Masters",
    items: [
      { label: "Chart of Accounts", href: "/gl/accounts", icon: BookOpen, shortcut: "Alt+A" },
      { label: "Vendors (AP)", href: "/ap/vendors", icon: Building2 },
      { label: "Fee Structures (AR)", href: "/ar/fee-structures", icon: Users },
    ]
  },
  {
    group: "Transactions",
    items: [
      { label: "Vouchers (Journal)", href: "/gl/journals", icon: FileText, shortcut: "Alt+V" },
      { label: "Purchase Orders", href: "/ap/purchase-orders", icon: ListOrdered },
      { label: "Fee Receipts", href: "/ar/payments/receipts", icon: FileText },
    ]
  },
  {
    group: "Reports",
    items: [
      { label: "Trial Balance", href: "/gl/reports/trial-balance", icon: PieChart, shortcut: "Alt+T" },
      { label: "P&L / Balance Sheet", href: "/reports", icon: PieChart },
    ]
  },
  {
    group: "System",
    items: [
      { label: "Settings", href: "/settings", icon: Settings },
    ]
  }
];

export function Sidebar() {
  const location = useLocation();
  const navigate = useNavigate();

  const { format } = useShortcutDisplay();

  // Register Global Shortcuts
  useKeyboardShortcut('v', () => navigate('/gl/journals'), { alt: true });
  useKeyboardShortcut('d', () => navigate('/dashboard'), { alt: true });
  useKeyboardShortcut('a', () => navigate('/gl/accounts'), { alt: true });
  useKeyboardShortcut('t', () => navigate('/gl/reports/trial-balance'), { alt: true });

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
        {navigation.map((group, idx) => (
          <div key={idx} className="mb-6">
            <h3 className="mb-2 px-3 text-[10px] font-bold uppercase tracking-widest text-slate-400/80">
              {group.group}
            </h3>
            <ul className="space-y-1">
              {group.items.map((item) => {
                const isActive = location.pathname.startsWith(item.href) &&
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
                      {isActive && (
                        <div className="absolute left-0 top-0 bottom-0 w-1 bg-primary rounded-r-full" />
                      )}
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
          </div>
        ))}
      </nav>

      {/* Footer */}
      <div className="border-t border-white/5 p-4 relative z-10 bg-black/20 backdrop-blur-md">
        <div className="flex items-center gap-2 text-[11px] font-medium text-slate-300/80">
          <div className="h-2 w-2 rounded-full bg-emerald-400 shadow-[0_0_10px_rgba(52,211,153,0.8)] animate-pulse" />
          Core Engine Connected
        </div>
      </div>
    </aside>
  );
}

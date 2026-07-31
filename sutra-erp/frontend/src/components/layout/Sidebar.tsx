import { NavLink, useLocation } from "react-router-dom";
import { cn } from "../../lib/utils";
import {
  LayoutDashboard,
  BookOpen,
  Users,
  Building2,
  Landmark,
  ReceiptIndianRupee,
  Settings,
  BarChart3,
  type LucideIcon,
} from "lucide-react";

interface NavItem {
  label: string;
  href: string;
  icon: LucideIcon;
  children?: NavItem[];
}

const navigation: NavItem[] = [
  { label: "Dashboard", href: "/dashboard", icon: LayoutDashboard },
  { label: "General Ledger", href: "/gl/accounts", icon: BookOpen },
  { label: "Accounts Receivable", href: "/ar/fee-structures", icon: Users },
  { label: "Accounts Payable", href: "/ap/vendors", icon: Building2 },
  { label: "Treasury", href: "/treasury/bank-accounts", icon: Landmark },
  { label: "Taxation", href: "/tax/gst/registrations", icon: ReceiptIndianRupee },
  { label: "Reports", href: "/reports", icon: BarChart3 },
  { label: "Settings", href: "/settings", icon: Settings },
];

export function Sidebar() {
  const location = useLocation();

  return (
    <aside className="flex w-60 flex-col border-r bg-card">
      {/* Brand */}
      <div className="flex h-14 items-center gap-2 border-b px-4">
        <div className="flex h-8 w-8 items-center justify-center rounded bg-primary font-bold text-primary-foreground">
          S
        </div>
        <span className="text-lg font-semibold tracking-tight">SutraERP</span>
      </div>

      {/* Navigation */}
      <nav className="flex-1 overflow-y-auto p-2">
        <ul className="space-y-1">
          {navigation.map((item) => {
            const isActive = location.pathname.startsWith(item.href) &&
              (item.href !== "/dashboard" || location.pathname === "/dashboard" || location.pathname === "/");
            return (
              <li key={item.href}>
                <NavLink
                  to={item.href}
                  className={cn(
                    "flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors",
                    isActive
                      ? "bg-primary/10 text-primary"
                      : "text-muted-foreground hover:bg-accent hover:text-accent-foreground"
                  )}
                >
                  <item.icon className="h-4 w-4" />
                  {item.label}
                </NavLink>
              </li>
            );
          })}
        </ul>
      </nav>

      {/* Footer */}
      <div className="border-t p-3">
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <div className="h-2 w-2 rounded-full bg-emerald-500" />
          v0.1.0 — Connected
        </div>
      </div>
    </aside>
  );
}

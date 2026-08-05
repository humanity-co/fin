import { type ReactNode } from "react";
import { cn } from "../../lib/utils";

interface AppShellProps {
  children: ReactNode;
  sidebar?: ReactNode;
  className?: string;
}

export function AppShell({ children, sidebar, className }: AppShellProps) {
  return (
    <div className={cn("flex h-screen overflow-hidden bg-transparent", className)}>
      {sidebar}
      <div className="flex flex-1 flex-col overflow-hidden relative">
        {/* Top Breadcrumb / Action Bar - Minimalist */}
        <header className="h-14 bg-white flex items-center justify-between px-6 shadow-sm z-10 shrink-0 sticky top-0 border-b border-border">
          <div className="flex items-center gap-2 text-xs font-medium text-slate-500">
            <span className="text-slate-800 font-semibold tracking-wide">Sutra Enterprise</span>
            <span className="text-slate-300">/</span>
            <span className="text-primary font-bold">Active Session</span>
          </div>
          <div className="flex items-center gap-4 text-xs font-medium">
            <div className="flex items-center gap-2 bg-slate-50 px-3 py-1.5 rounded-full text-slate-700 border border-slate-200">
              <span className="relative flex h-2 w-2">
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
                <span className="relative inline-flex rounded-full h-2 w-2 bg-emerald-500"></span>
              </span>
              FY 2026-27
            </div>
            <div className="h-8 w-8 rounded-full bg-primary flex items-center justify-center text-white font-bold hover-lift cursor-pointer shadow-sm">
              A
            </div>
          </div>
        </header>

        <main className="flex-1 overflow-y-auto p-8 relative">
          <div className="mx-auto max-w-7xl h-full animate-in fade-in slide-in-from-bottom-4 duration-500">
            {children}
          </div>
        </main>
      </div>
    </div>
  );
}

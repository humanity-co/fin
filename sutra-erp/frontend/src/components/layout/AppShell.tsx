import { type ReactNode } from "react";
import { cn } from "../../lib/utils";

interface AppShellProps {
  children: ReactNode;
  sidebar?: ReactNode;
  className?: string;
}

export function AppShell({ children, sidebar, className }: AppShellProps) {
  return (
    <div className={cn("flex h-screen overflow-hidden bg-background", className)}>
      {sidebar}
      <div className="flex flex-1 flex-col overflow-hidden">
        <main className="flex-1 overflow-y-auto p-6">{children}</main>
      </div>
    </div>
  );
}

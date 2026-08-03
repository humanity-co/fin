import { useMemo } from "react";
import type { ComponentType } from "react";
import CfoDashboard from "../pages/dashboard/CfoDashboard";
import { PrincipalDashboard, AccountantDashboard, HodDashboard, StudentDashboard, AuditorDashboard, GenericDashboard } from "../pages/dashboard/roleDashboards";

export type DashboardRole = "CFO" | "Principal" | "Accountant" | "HOD" | "Student" | "Parent" | "Auditor" | "Other";

/** Reads the auth payload written by the shell. Supports roles as strings or {name} grants. */
export function useCurrentRoles(): string[] {
  return useMemo(() => {
    try {
      const raw = localStorage.getItem("sutra-auth");
      if (!raw) return ["CFO"];
      const user = JSON.parse(raw) as { roles?: unknown; role?: unknown };
      const roles = user.roles ?? user.role ?? [];
      return (Array.isArray(roles) ? roles : [roles]).map(role => typeof role === "string" ? role : (role as { name?: string })?.name ?? "").filter(Boolean);
    } catch { return ["CFO"]; }
  }, []);
}

export function useDashboardForRole(): ComponentType {
  const roles = useCurrentRoles().map(role => role.toLowerCase());
  if (roles.includes("cfo")) return CfoDashboard;
  if (roles.includes("principal") || roles.includes("director")) return PrincipalDashboard;
  if (roles.includes("accountant")) return AccountantDashboard;
  if (roles.includes("hod")) return HodDashboard;
  if (roles.includes("student") || roles.includes("parent")) return StudentDashboard;
  if (roles.includes("auditor") || roles.includes("internal auditor") || roles.includes("external auditor")) return AuditorDashboard;
  return GenericDashboard;
}

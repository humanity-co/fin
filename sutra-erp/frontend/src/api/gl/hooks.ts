import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api, type PaginatedResponse, buildQueryParams } from "../../lib/api-client";

// ── Types ──────────────────────────────────

export interface Account {
  accountId: string;
  accountCode: string;
  accountName: string;
  accountType: string;
  parentAccountId: string | null;
  level: number;
  gstClassification: string;
  hsnSacCode: string | null;
  itcEligibility: string;
  aisheHeadCode: string | null;
  isActive: boolean;
  openingBalance: number;
  currentBalance: number;
}

export interface Journal {
  journalId: string;
  journalNumber: string;
  journalType: string;
  postingDate: string;
  description: string;
  status: string;
  totalDebit: number;
  totalCredit: number;
  lines: JournalLine[];
}

export interface JournalLine {
  journalLineId: string;
  lineNumber: number;
  accountId: string;
  accountName?: string;
  accountCode?: string;
  debitAmount: number | null;
  creditAmount: number | null;
  description: string;
}

// ── Keys ───────────────────────────────────

export const glKeys = {
  all: ["gl"] as const,
  accounts: () => [...glKeys.all, "accounts"] as const,
  account: (id: string) => [...glKeys.accounts(), id] as const,
  accountTree: () => [...glKeys.accounts(), "tree"] as const,
  journals: (filters?: Record<string, string>) => [...glKeys.all, "journals", filters] as const,
  journal: (id: string) => [...glKeys.all, "journals", id] as const,
  trialBalance: (params?: Record<string, string>) => [...glKeys.all, "trial-balance", params] as const,
};

// ── Hooks ──────────────────────────────────

export function useAccounts(params?: Record<string, string>) {
  return useQuery({
    queryKey: glKeys.accounts(),
    queryFn: () =>
      api.get<PaginatedResponse<Account>>(
        `/accounts${buildQueryParams(params || {})}`
      ),
  });
}

export function useAccountTree() {
  return useQuery({
    queryKey: glKeys.accountTree(),
    queryFn: () => api.get<Account[]>("/accounts"),
  });
}

export function useAccount(id: string) {
  return useQuery({
    queryKey: glKeys.account(id),
    queryFn: () => api.get<Account>(`/accounts/${id}`),
    enabled: !!id,
  });
}

export function useJournals(params?: Record<string, string>) {
  return useQuery({
    queryKey: glKeys.journals(params),
    queryFn: () =>
      api.get<PaginatedResponse<Journal>>(
        `/journals${buildQueryParams(params || {})}`
      ),
  });
}

export function useJournal(id: string) {
  return useQuery({
    queryKey: glKeys.journal(id),
    queryFn: () => api.get<Journal>(`/journals/${id}`),
    enabled: !!id,
  });
}

export function useTrialBalance(params?: Record<string, string>) {
  return useQuery({
    queryKey: glKeys.trialBalance(params),
    queryFn: () =>
      api.get(`/reports/trial-balance${buildQueryParams(params || {})}`),
  });
}

export function useCreateJournal() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: unknown) => api.post("/journals", data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: glKeys.journals() });
    },
  });
}

export function usePostJournal() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.post(`/journals/${id}/post`),
    onSuccess: (_, id) => {
      qc.invalidateQueries({ queryKey: glKeys.journal(id) });
      qc.invalidateQueries({ queryKey: glKeys.journals() });
    },
  });
}

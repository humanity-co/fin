import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api, type PaginatedResponse, buildQueryParams } from "../../lib/api-client";

export interface Vendor {
  vendorId: string;
  vendorCode: string;
  vendorName: string;
  vendorType: string;
  pan: string;
  panStatus: string;
  gstin: string | null;
  gstinStatus: string;
  isActive: boolean;
  isBlacklisted: boolean;
  paymentTerms: number;
}

export interface PurchaseOrder {
  purchaseOrderId: string;
  poNumber: string;
  vendorId: string;
  vendorName?: string;
  orderDate: string;
  deliveryDate: string | null;
  status: string;
  totalAmount: number;
  taxAmount: number;
  netAmount: number;
  isRcmApplicable: boolean;
}

export const apKeys = {
  all: ["ap"] as const,
  vendors: (filters?: Record<string, string>) => [...apKeys.all, "vendors", filters] as const,
  vendor: (id: string) => [...apKeys.all, "vendors", id] as const,
  purchaseOrders: (filters?: Record<string, string>) => [...apKeys.all, "pos", filters] as const,
  purchaseOrder: (id: string) => [...apKeys.all, "pos", id] as const,
};

export function useVendors(params?: Record<string, string>) {
  return useQuery({
    queryKey: apKeys.vendors(params),
    queryFn: () =>
      api.get<PaginatedResponse<Vendor>>(
        `/vendors${buildQueryParams(params || {})}`
      ),
  });
}

export function useVendor(id: string) {
  return useQuery({
    queryKey: apKeys.vendor(id),
    queryFn: () => api.get<Vendor>(`/vendors/${id}`),
    enabled: !!id,
  });
}

export function useCreateVendor() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: unknown) => api.post("/vendors", data),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: apKeys.vendors() });
    },
  });
}

export function usePurchaseOrders(params?: Record<string, string>) {
  return useQuery({
    queryKey: apKeys.purchaseOrders(params),
    queryFn: () =>
      api.get<PaginatedResponse<PurchaseOrder>>(
        `/purchase-orders${buildQueryParams(params || {})}`
      ),
  });
}

export function usePurchaseOrder(id: string) {
  return useQuery({
    queryKey: apKeys.purchaseOrder(id),
    queryFn: () => api.get<PurchaseOrder>(`/purchase-orders/${id}`),
    enabled: !!id,
  });
}

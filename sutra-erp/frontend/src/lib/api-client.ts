/**
 * API client for SutraERP backend.
 * Wraps fetch with base URL, tenant header, and error handling.
 */

const API_BASE_URL =
  import.meta.env.VITE_API_BASE_URL || "http://localhost:8080/api/v1";

const TENANT_ID_HEADER = "X-Tenant-Id";

interface ApiError {
  status: number;
  code: string;
  message: string;
  details?: Record<string, string[]>;
}

export class ApiRequestError extends Error {
  status: number;
  code: string;
  details?: Record<string, string[]>;

  constructor(error: ApiError) {
    super(error.message);
    this.name = "ApiRequestError";
    this.status = error.status;
    this.code = error.code;
    this.details = error.details;
  }
}

let tenantId: string | null = null;

export function setTenantId(id: string) {
  tenantId = id;
}

export function getTenantId(): string | null {
  return tenantId;
}

async function buildHeaders(init?: RequestInit): Promise<HeadersInit> {
  const headers: HeadersInit = {
    "Content-Type": "application/json",
    ...(init?.headers as Record<string, string>),
  };

  if (tenantId) {
    (headers as Record<string, string>)[TENANT_ID_HEADER] = tenantId;
  }

  return headers;
}

export async function apiFetch<T = unknown>(
  path: string,
  init?: RequestInit
): Promise<T> {
  const url = `${API_BASE_URL}${path}`;
  const headers = await buildHeaders(init);

  const response = await fetch(url, {
    ...init,
    headers,
  });

  if (!response.ok) {
    let errorBody: ApiError;
    try {
      errorBody = await response.json();
    } catch {
      errorBody = {
        status: response.status,
        code: "UNKNOWN_ERROR",
        message: response.statusText || "An unexpected error occurred",
      };
    }
    throw new ApiRequestError(errorBody);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  const json = await response.json();
  if (json && typeof json === "object" && "success" in json) {
    if (json.success) {
      return json.data as T;
    } else {
      throw new ApiRequestError({
        status: response.status,
        code: "APP_ERROR",
        message: json.error || "Unknown application error",
      });
    }
  }
  return json as T;
}

// Convenience methods
export const api = {
  get<T = unknown>(path: string, init?: RequestInit) {
    return apiFetch<T>(path, { ...init, method: "GET" });
  },

  post<T = unknown>(path: string, body?: unknown, init?: RequestInit) {
    return apiFetch<T>(path, {
      ...init,
      method: "POST",
      body: body ? JSON.stringify(body) : undefined,
    });
  },

  put<T = unknown>(path: string, body?: unknown, init?: RequestInit) {
    return apiFetch<T>(path, {
      ...init,
      method: "PUT",
      body: body ? JSON.stringify(body) : undefined,
    });
  },

  patch<T = unknown>(path: string, body?: unknown, init?: RequestInit) {
    return apiFetch<T>(path, {
      ...init,
      method: "PATCH",
      body: body ? JSON.stringify(body) : undefined,
    });
  },

  delete<T = unknown>(path: string, init?: RequestInit) {
    return apiFetch<T>(path, { ...init, method: "DELETE" });
  },
};

// Paginated response type
export interface PaginatedResponse<T> {
  data: T[];
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

// Query params builder
export function buildQueryParams(
  params: Record<string, string | number | boolean | undefined | null>
): string {
  const searchParams = new URLSearchParams();
  Object.entries(params).forEach(([key, value]) => {
    if (value !== undefined && value !== null && value !== "") {
      searchParams.set(key, String(value));
    }
  });
  const qs = searchParams.toString();
  return qs ? `?${qs}` : "";
}

import { lazy, Suspense } from "react";
import { Routes, Route } from "react-router-dom";
import { AppShell } from "../components/layout/AppShell";
import { Sidebar } from "../components/layout/Sidebar";
import { Spinner } from "../components/ui";
import { useDashboardForRole } from "../hooks/useDashboardRole";

// ── Lazy-loaded route components ────────────

const CfoDashboard = lazy(() => import("../pages/dashboard/CfoDashboard"));
const ComplianceCalendar = lazy(() => import("../pages/dashboard/ComplianceCalendar"));

const ChartOfAccounts = lazy(() => import("../pages/gl/ChartOfAccounts"));
const JournalList = lazy(() => import("../pages/gl/JournalList"));
const JournalEntry = lazy(() => import("../pages/gl/JournalEntry"));
const TrialBalance = lazy(() => import("../pages/gl/TrialBalance"));

const FeeStructures = lazy(() => import("../pages/ar/FeeStructures"));
const FeeCollection = lazy(() => import("../pages/ar/FeeCollection"));
const Scholarships = lazy(() => import("../pages/ar/Scholarships"));
const Refunds = lazy(() => import("../pages/ar/Refunds"));

const Vendors = lazy(() => import("../pages/ap/Vendors"));
const PurchaseOrders = lazy(() => import("../pages/ap/PurchaseOrders"));
const PurchaseInvoices = lazy(() => import("../pages/ap/PurchaseInvoices"));
const Payments = lazy(() => import("../pages/ap/Payments"));

const BankAccounts = lazy(() => import("../pages/treasury/BankAccounts"));
const Reconciliation = lazy(() => import("../pages/treasury/Reconciliation"));

const GstReports = lazy(() => import("../pages/tax/GstReports"));
const TdsDeductions = lazy(() => import("../pages/tax/TdsDeductions"));

const NaacDashboard = lazy(() => import("../pages/reports/NaacDashboard"));

const SystemSettings = lazy(() => import("../pages/settings/SystemSettings"));
const UserManagement = lazy(() => import("../pages/settings/UserManagement"));

// ── Loading fallback ────────────────────────

function PageLoader() {
  return (
    <div className="flex h-64 items-center justify-center">
      <Spinner className="h-8 w-8 text-primary" />
    </div>
  );
}

function LazyPage({ children }: { children: React.ReactNode }) {
  return <Suspense fallback={<PageLoader />}>{children}</Suspense>;
}

// ── Shell layout ────────────────────────────

function Shell({ children }: { children: React.ReactNode }) {
  return (
    <AppShell sidebar={<Sidebar />}>
      {children}
    </AppShell>
  );
}

function RoleDashboard() {
  const Dashboard = useDashboardForRole();
  return <Dashboard />;
}

// ── Route definitions ───────────────────────

export function AppRoutes() {
  return (
    <Routes>
      <Route
        path="/"
        element={
          <Shell>
            <LazyPage><CfoDashboard /></LazyPage>
          </Shell>
        }
      />
      <Route
        path="/dashboard"
        element={
          <Shell>
            <LazyPage><RoleDashboard /></LazyPage>
          </Shell>
        }
      />
      <Route
        path="/dashboard/compliance"
        element={
          <Shell>
            <LazyPage><ComplianceCalendar /></LazyPage>
          </Shell>
        }
      />

      {/* General Ledger */}
      <Route
        path="/gl/accounts"
        element={
          <Shell>
            <LazyPage><ChartOfAccounts /></LazyPage>
          </Shell>
        }
      />
      <Route
        path="/gl/journals"
        element={
          <Shell>
            <LazyPage><JournalList /></LazyPage>
          </Shell>
        }
      />
      <Route
        path="/gl/journals/new"
        element={
          <Shell>
            <LazyPage><JournalEntry /></LazyPage>
          </Shell>
        }
      />
      <Route
        path="/gl/journals/:id"
        element={
          <Shell>
            <LazyPage><JournalEntry /></LazyPage>
          </Shell>
        }
      />
      <Route
        path="/gl/reports/trial-balance"
        element={
          <Shell>
            <LazyPage><TrialBalance /></LazyPage>
          </Shell>
        }
      />

      {/* Accounts Receivable */}
      <Route
        path="/ar/fee-structures"
        element={
          <Shell>
            <LazyPage><FeeStructures /></LazyPage>
          </Shell>
        }
      />
      <Route
        path="/ar/payments/receipts"
        element={
          <Shell>
            <LazyPage><FeeCollection /></LazyPage>
          </Shell>
        }
      />
      <Route
        path="/ar/scholarships"
        element={
          <Shell>
            <LazyPage><Scholarships /></LazyPage>
          </Shell>
        }
      />
      <Route
        path="/ar/refunds"
        element={
          <Shell>
            <LazyPage><Refunds /></LazyPage>
          </Shell>
        }
      />

      {/* Accounts Payable */}
      <Route
        path="/ap/vendors"
        element={
          <Shell>
            <LazyPage><Vendors /></LazyPage>
          </Shell>
        }
      />
      <Route
        path="/ap/purchase-orders"
        element={
          <Shell>
            <LazyPage><PurchaseOrders /></LazyPage>
          </Shell>
        }
      />
      <Route
        path="/ap/purchase-invoices"
        element={
          <Shell>
            <LazyPage><PurchaseInvoices /></LazyPage>
          </Shell>
        }
      />
      <Route
        path="/ap/payments"
        element={
          <Shell>
            <LazyPage><Payments /></LazyPage>
          </Shell>
        }
      />

      {/* Treasury */}
      <Route
        path="/treasury/bank-accounts"
        element={
          <Shell>
            <LazyPage><BankAccounts /></LazyPage>
          </Shell>
        }
      />
      <Route
        path="/treasury/reconciliation"
        element={
          <Shell>
            <LazyPage><Reconciliation /></LazyPage>
          </Shell>
        }
      />

      {/* Taxation */}
      <Route
        path="/tax/gst/registrations"
        element={
          <Shell>
            <LazyPage><GstReports /></LazyPage>
          </Shell>
        }
      />
      <Route
        path="/tax/tds/deductions"
        element={
          <Shell>
            <LazyPage><TdsDeductions /></LazyPage>
          </Shell>
        }
      />

      {/* Reports */}
      <Route
        path="/reports"
        element={
          <Shell>
            <LazyPage><NaacDashboard /></LazyPage>
          </Shell>
        }
      />

      {/* Settings */}
      <Route
        path="/settings"
        element={
          <Shell>
            <LazyPage><SystemSettings /></LazyPage>
          </Shell>
        }
      />
      <Route
        path="/settings/users"
        element={
          <Shell>
            <LazyPage><UserManagement /></LazyPage>
          </Shell>
        }
      />
    </Routes>
  );
}
/home/agent-lead/.profile: line 28: /home/agent-lead/.cargo/env: No such file or directory
/home/agent-lead/.profile: line 28: /home/agent-lead/.cargo/env: No such file or directory
/home/agent-lead/.profile: line 28: /home/agent-lead/.cargo/env: No such file or directory

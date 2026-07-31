import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent } from "../../components/ui";

export default function PurchaseInvoices() {
  return (
    <div>
      <PageHeader
        title="Purchase Invoices"
        description="Record and match vendor invoices"
        breadcrumbs={[{ label: "Accounts Payable", href: "/ap/vendors" }, { label: "Invoices" }]}
      />
      <Card>
        <CardContent className="flex items-center justify-center py-12 text-sm text-muted-foreground">
          <div className="text-center">
            <p className="font-medium mb-1">Purchase Invoices — Coming Soon</p>
            <p>3-way matching (PO vs GRN vs Invoice), TDS auto-calculation, GST ITC tracking, and approval workflow.</p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

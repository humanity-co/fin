import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent } from "../../components/ui";

export default function Payments() {
  return (
    <div>
      <PageHeader
        title="Vendor Payments"
        description="Process and track vendor payments"
        breadcrumbs={[{ label: "Accounts Payable", href: "/ap/vendors" }, { label: "Payments" }]}
      />
      <Card>
        <CardContent className="flex items-center justify-center py-12 text-sm text-muted-foreground">
          <div className="text-center">
            <p className="font-medium mb-1">Vendor Payments — Coming Soon</p>
            <p>Payment initiation, approval workflow, TDS deduction at source, and bank integration.</p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

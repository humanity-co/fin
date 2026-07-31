import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button } from "../../components/ui";
import { Plus } from "lucide-react";

export default function PurchaseOrders() {
  return (
    <div>
      <PageHeader
        title="Purchase Orders"
        description="Create and manage purchase orders"
        breadcrumbs={[{ label: "Accounts Payable", href: "/ap/vendors" }, { label: "Purchase Orders" }]}
        actions={
          <Button size="sm">
            <Plus className="h-4 w-4 mr-1" /> New PO
          </Button>
        }
      />
      <Card>
        <CardContent className="flex items-center justify-center py-12 text-sm text-muted-foreground">
          <div className="text-center">
            <p className="font-medium mb-1">Purchase Orders — Coming Soon</p>
            <p>Vendor search with verification badges, RCM auto-detection, line items with HSN/tax, budget availability check.</p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

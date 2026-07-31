import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button } from "../../components/ui";
import { Plus } from "lucide-react";

export default function FeeCollection() {
  return (
    <div>
      <PageHeader
        title="Fee Collection"
        description="Record and manage student fee payments"
        breadcrumbs={[{ label: "Accounts Receivable", href: "/ar/fee-structures" }, { label: "Fee Collection" }]}
        actions={
          <Button size="sm">
            <Plus className="h-4 w-4 mr-1" /> Record Payment
          </Button>
        }
      />
      <Card>
        <CardContent className="flex items-center justify-center py-12 text-sm text-muted-foreground">
          <div className="text-center">
            <p className="font-medium mb-1">Fee Collection — Coming Soon</p>
            <p>Student lookup, payment recording, allocation to installments, gateway integration, and receipt generation.</p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

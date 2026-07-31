import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button } from "../../components/ui";
import { Plus } from "lucide-react";

export default function Refunds() {
  return (
    <div>
      <PageHeader
        title="Refunds"
        description="Process and track student fee refunds"
        breadcrumbs={[{ label: "Accounts Receivable", href: "/ar/fee-structures" }, { label: "Refunds" }]}
        actions={
          <Button size="sm">
            <Plus className="h-4 w-4 mr-1" /> Initiate Refund
          </Button>
        }
      />
      <Card>
        <CardContent className="flex items-center justify-center py-12 text-sm text-muted-foreground">
          <div className="text-center">
            <p className="font-medium mb-1">Refunds — Coming Soon</p>
            <p>FRC-compliant refund calculation, credit notes, approval workflow, and bank processing.</p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

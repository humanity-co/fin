import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button } from "../../components/ui";
import { Plus } from "lucide-react";

export default function ChartOfAccounts() {
  return (
    <div>
      <PageHeader
        title="Chart of Accounts"
        description="Manage the general ledger account hierarchy"
        breadcrumbs={[{ label: "General Ledger", href: "/gl/accounts" }]}
        actions={
          <Button size="sm">
            <Plus className="h-4 w-4 mr-1" /> New Account
          </Button>
        }
      />

      <Card>
        <CardContent className="flex items-center justify-center py-12 text-sm text-muted-foreground">
          <div className="text-center">
            <p className="font-medium mb-1">Chart of Accounts — Coming Soon</p>
            <p>5-level COA hierarchy with AISHE/NAAC mapping, GST classification, and ITC eligibility tagging.</p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

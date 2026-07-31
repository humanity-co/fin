import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent } from "../../components/ui";

export default function TrialBalance() {
  return (
    <div>
      <PageHeader
        title="Trial Balance"
        description="View trial balance for the selected period"
        breadcrumbs={[
          { label: "General Ledger", href: "/gl/accounts" },
          { label: "Reports", href: "#" },
          { label: "Trial Balance" },
        ]}
      />

      <Card>
        <CardContent className="flex items-center justify-center py-12 text-sm text-muted-foreground">
          <div className="text-center">
            <p className="font-medium mb-1">Trial Balance — Coming Soon</p>
            <p>Collapsible hierarchy with opening/closing balance columns, period selector, and entity filter.</p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

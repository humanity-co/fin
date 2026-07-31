import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent } from "../../components/ui";

export default function Reconciliation() {
  return (
    <div>
      <PageHeader
        title="Bank Reconciliation"
        description="Reconcile bank statements with system transactions"
        breadcrumbs={[{ label: "Treasury", href: "/treasury/bank-accounts" }, { label: "Reconciliation" }]}
      />
      <Card>
        <CardContent className="flex items-center justify-center py-12 text-sm text-muted-foreground">
          <div className="text-center">
            <p className="font-medium mb-1">Bank Reconciliation — Coming Soon</p>
            <p>Split-view workspace with auto-match suggestions, manual match flow, and difference analysis.</p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

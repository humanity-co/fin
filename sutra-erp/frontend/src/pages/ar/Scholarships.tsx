import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent } from "../../components/ui";

export default function Scholarships() {
  return (
    <div>
      <PageHeader
        title="Scholarships"
        description="Manage student scholarships and DBT reconciliation"
        breadcrumbs={[{ label: "Accounts Receivable", href: "/ar/fee-structures" }, { label: "Scholarships" }]}
      />
      <Card>
        <CardContent className="flex items-center justify-center py-12 text-sm text-muted-foreground">
          <div className="text-center">
            <p className="font-medium mb-1">Scholarship Management — Coming Soon</p>
            <p>Lifecycle tracking: Applied → Verified → Sanctioned → Disbursed → Reconciled. MahaDBT integration with DBT reconciliation.</p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

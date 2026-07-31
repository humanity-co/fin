import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent } from "../../components/ui";

export default function AisheExtract() {
  return (
    <div>
      <PageHeader
        title="AISHE Data Extract"
        description="Generate AISHE-compliant financial data"
        breadcrumbs={[{ label: "Reports", href: "/reports" }, { label: "AISHE Extract" }]}
      />
      <Card>
        <CardContent className="flex items-center justify-center py-12 text-sm text-muted-foreground">
          <div className="text-center">
            <p className="font-medium mb-1">AISHE Extract — Coming Soon</p>
            <p>Auto-generated AISHE financial data from COA mappings. Export as CSV/JSON for AISHE portal submission.</p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

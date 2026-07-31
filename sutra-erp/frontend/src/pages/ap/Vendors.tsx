import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button } from "../../components/ui";
import { Plus } from "lucide-react";

export default function Vendors() {
  return (
    <div>
      <PageHeader
        title="Vendor Master"
        description="Manage vendor onboarding, verification, and lifecycle"
        breadcrumbs={[{ label: "Accounts Payable", href: "/ap/vendors" }]}
        actions={
          <Button size="sm">
            <Plus className="h-4 w-4 mr-1" /> Onboard Vendor
          </Button>
        }
      />
      <Card>
        <CardContent className="flex items-center justify-center py-12 text-sm text-muted-foreground">
          <div className="text-center">
            <p className="font-medium mb-1">Vendor Master — Coming Soon</p>
            <p>PAN/GSTIN verification, Section 197 certificates, bank validation, blacklisting, and TDS rate configuration.</p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

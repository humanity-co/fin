import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent } from "../../components/ui";

export default function TdsDeductions() {
  return (
    <div>
      <PageHeader
        title="TDS Deductions"
        description="Track TDS deductions and generate returns"
        breadcrumbs={[{ label: "Taxation", href: "/tax/gst/registrations" }, { label: "TDS" }]}
      />
      <Card>
        <CardContent className="flex items-center justify-center py-12 text-sm text-muted-foreground">
          <div className="text-center">
            <p className="font-medium mb-1">TDS Deductions — Coming Soon</p>
            <p>Section-wise deduction register, Section 197 certificate management, Form 16/16A generation.</p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

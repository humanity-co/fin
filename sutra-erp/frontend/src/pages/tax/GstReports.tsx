import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent } from "../../components/ui";

export default function GstReports() {
  return (
    <div>
      <PageHeader
        title="GST Reports"
        description="GST registrations, returns, and ITC register"
        breadcrumbs={[{ label: "Taxation", href: "/tax/gst/registrations" }]}
      />
      <Card>
        <CardContent className="flex items-center justify-center py-12 text-sm text-muted-foreground">
          <div className="text-center">
            <p className="font-medium mb-1">GST Reports — Coming Soon</p>
            <p>Entity selector, period picker, tabbed GSTR-1/3B/9 views, ITC register with Rule 42/43 reversals.</p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

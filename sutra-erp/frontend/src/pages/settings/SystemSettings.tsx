import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent } from "../../components/ui";

export default function SystemSettings() {
  return (
    <div>
      <PageHeader
        title="System Settings"
        description="Configure system preferences and policies"
        breadcrumbs={[{ label: "Settings" }]}
      />
      <Card>
        <CardContent className="flex items-center justify-center py-12 text-sm text-muted-foreground">
          <div className="text-center">
            <p className="font-medium mb-1">System Settings — Coming Soon</p>
            <p>Tenant configuration, policy management, entity setup, fiscal year management, and compliance calendar.</p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

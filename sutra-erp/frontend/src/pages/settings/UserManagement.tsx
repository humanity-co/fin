import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent } from "../../components/ui";

export default function UserManagement() {
  return (
    <div>
      <PageHeader
        title="User Management"
        description="Manage users, roles, and permissions"
        breadcrumbs={[{ label: "Settings", href: "/settings" }, { label: "Users" }]}
      />
      <Card>
        <CardContent className="flex items-center justify-center py-12 text-sm text-muted-foreground">
          <div className="text-center">
            <p className="font-medium mb-1">User Management — Coming Soon</p>
            <p>Role-based access control: CFO, Controller, Accountant, Cashier, Registrar, Auditor with granular permissions.</p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

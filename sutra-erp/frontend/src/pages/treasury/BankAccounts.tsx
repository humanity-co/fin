import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button } from "../../components/ui";
import { Plus } from "lucide-react";

export default function BankAccounts() {
  return (
    <div>
      <PageHeader
        title="Bank Accounts"
        description="Manage linked bank accounts"
        breadcrumbs={[{ label: "Treasury", href: "/treasury/bank-accounts" }]}
        actions={
          <Button size="sm">
            <Plus className="h-4 w-4 mr-1" /> Add Bank Account
          </Button>
        }
      />
      <Card>
        <CardContent className="flex items-center justify-center py-12 text-sm text-muted-foreground">
          <div className="text-center">
            <p className="font-medium mb-1">Bank Accounts — Coming Soon</p>
            <p>Bank account management with IFSC validation, balance tracking, and payment gateway configuration.</p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

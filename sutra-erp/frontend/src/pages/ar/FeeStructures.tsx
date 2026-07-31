import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button } from "../../components/ui";
import { Plus } from "lucide-react";

export default function FeeStructures() {
  return (
    <div>
      <PageHeader
        title="Fee Structures"
        description="Define and manage fee structures by program, year, and category"
        breadcrumbs={[{ label: "Accounts Receivable", href: "/ar/fee-structures" }]}
        actions={
          <Button size="sm">
            <Plus className="h-4 w-4 mr-1" /> New Fee Structure
          </Button>
        }
      />
      <Card>
        <CardContent className="flex items-center justify-center py-12 text-sm text-muted-foreground">
          <div className="text-center">
            <p className="font-medium mb-1">Fee Structures — Coming Soon</p>
            <p>Program → Academic Year → Semester → Fee Category → Fee Head hierarchy with GST classification, FRC approval tracking, and installment plans.</p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

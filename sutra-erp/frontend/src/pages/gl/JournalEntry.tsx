import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent } from "../../components/ui";

export default function JournalEntry() {
  return (
    <div>
      <PageHeader
        title="New Journal Entry"
        description="Create a double-entry journal"
        breadcrumbs={[
          { label: "General Ledger", href: "/gl/accounts" },
          { label: "Journals", href: "/gl/journals" },
          { label: "New Entry" },
        ]}
      />

      <Card>
        <CardContent className="flex items-center justify-center py-12 text-sm text-muted-foreground">
          <div className="text-center">
            <p className="font-medium mb-1">Journal Entry Form — Coming Soon</p>
            <p>Dual-list debit/credit layout with auto-balance indicator. Types: Standard, Reversing, Adjustment, RCM, TDS, Accrual, Prepayment.</p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

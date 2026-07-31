import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button } from "../../components/ui";
import { Plus } from "lucide-react";

export default function JournalList() {
  return (
    <div>
      <PageHeader
        title="Journal Entries"
        description="View, create, and manage journal entries"
        breadcrumbs={[{ label: "General Ledger", href: "/gl/accounts" }, { label: "Journals" }]}
        actions={
          <Button size="sm">
            <Plus className="h-4 w-4 mr-1" /> New Journal
          </Button>
        }
      />

      <Card>
        <CardContent className="flex items-center justify-center py-12 text-sm text-muted-foreground">
          <div className="text-center">
            <p className="font-medium mb-1">Journal List — Coming Soon</p>
            <p>Paginated journal entries with filtering by date, type, status, and amount.</p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

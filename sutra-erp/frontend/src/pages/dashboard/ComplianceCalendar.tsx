import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent } from "../../components/ui";
import { StatusBadge } from "../../components/data/StatusBadge";
import { IndianDate } from "../../components/data/IndianDate";

const deadlines = [
  { event: "GSTR-1 Filing", due: "2026-08-11", status: "Due Soon" },
  { event: "GSTR-3B Filing", due: "2026-08-20", status: "Upcoming" },
  { event: "TDS Return (Q1)", due: "2026-07-31", status: "Due Soon" },
  { event: "PF Payment", due: "2026-08-15", status: "Upcoming" },
  { event: "Income Tax Advance (Q2)", due: "2026-09-15", status: "Upcoming" },
];

export default function ComplianceCalendar() {
  return (
    <div>
      <PageHeader
        title="Compliance Calendar"
        description="Upcoming statutory deadlines for FY 2026-27"
        breadcrumbs={[
          { label: "Dashboard", href: "/dashboard" },
          { label: "Compliance" },
        ]}
      />

      <Card>
        <CardContent className="pt-6">
          <div className="space-y-4">
            {deadlines.map((d) => (
              <div
                key={d.event}
                className="flex items-center justify-between border-b pb-3 last:border-0"
              >
                <div>
                  <p className="font-medium">{d.event}</p>
                  <p className="text-sm text-muted-foreground">
                    Due: <IndianDate date={d.due} />
                  </p>
                </div>
                <StatusBadge status={d.status} />
              </div>
            ))}
          </div>
        </CardContent>
      </Card>

      <Card className="mt-4 border-dashed">
        <CardContent className="flex items-center justify-center py-8 text-sm text-muted-foreground">
          Full compliance calendar with month navigator and swimlane layout coming soon.
        </CardContent>
      </Card>
    </div>
  );
}

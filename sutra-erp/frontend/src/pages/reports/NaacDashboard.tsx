import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent } from "../../components/ui";
import { MoneyDisplay } from "../../components/data/MoneyDisplay";

export default function NaacDashboard() {
  return (
    <div>
      <PageHeader
        title="NAAC Dashboard"
        description="Financial metrics for NAAC accreditation"
        breadcrumbs={[{ label: "Reports" }]}
      />
      <div className="mb-6 grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <Card>
          <CardContent className="pt-6">
            <p className="text-sm text-muted-foreground">Research Grants</p>
            <p className="text-xl font-bold"><MoneyDisplay amount={2_50_00_000} /></p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <p className="text-sm text-muted-foreground">Grants per Faculty</p>
            <p className="text-xl font-bold"><MoneyDisplay amount={8_50_000} /></p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <p className="text-sm text-muted-foreground">Consultancy Revenue</p>
            <p className="text-xl font-bold"><MoneyDisplay amount={45_00_000} /></p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <p className="text-sm text-muted-foreground">Scholarship Expenditure</p>
            <p className="text-xl font-bold"><MoneyDisplay amount={1_20_00_000} /></p>
          </CardContent>
        </Card>
      </div>
      <Card>
        <CardContent className="flex items-center justify-center py-8 text-sm text-muted-foreground">
          Full NAAC dashboard with 5-year trend analysis, metric-wise breakdown, and export coming soon.
        </CardContent>
      </Card>
    </div>
  );
}

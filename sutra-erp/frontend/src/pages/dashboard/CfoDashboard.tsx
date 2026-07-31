import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui";
import { MoneyDisplay } from "../../components/data/MoneyDisplay";
import {
  TrendingUp,
  TrendingDown,
  IndianRupee,
  AlertTriangle,
  CheckCircle,
  Clock,
} from "lucide-react";

export default function CfoDashboard() {
  return (
    <div>
      <PageHeader
        title="CFO Dashboard"
        description="Financial overview for the current fiscal year"
        breadcrumbs={[{ label: "Dashboard" }]}
      />

      {/* KPI Cards */}
      <div className="mb-6 grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              Total Revenue
            </CardTitle>
            <TrendingUp className="h-4 w-4 text-emerald-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              <MoneyDisplay amount={15_67_34_500} />
            </div>
            <p className="text-xs text-muted-foreground">+12.5% from last year</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              Total Expenditure
            </CardTitle>
            <TrendingDown className="h-4 w-4 text-rose-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              <MoneyDisplay amount={12_34_56_700} />
            </div>
            <p className="text-xs text-muted-foreground">+8.3% from last year</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              Net Surplus
            </CardTitle>
            <IndianRupee className="h-4 w-4 text-blue-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              <MoneyDisplay amount={3_32_77_800} />
            </div>
            <p className="text-xs text-muted-foreground">21.2% of revenue</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              Collection Rate
            </CardTitle>
            <CheckCircle className="h-4 w-4 text-emerald-500" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">87.5%</div>
            <p className="text-xs text-muted-foreground">Target: 90%</p>
          </CardContent>
        </Card>
      </div>

      {/* Quick Stats */}
      <div className="mb-6 grid gap-4 md:grid-cols-3">
        <Card>
          <CardContent className="pt-6">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm text-muted-foreground">Bank Balance</p>
                <p className="text-xl font-bold">
                  <MoneyDisplay amount={8_45_23_000} />
                </p>
              </div>
              <IndianRupee className="h-8 w-8 text-muted-foreground/30" />
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardContent className="pt-6">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm text-muted-foreground">Pending Approvals</p>
                <p className="text-xl font-bold">12</p>
              </div>
              <Clock className="h-8 w-8 text-amber-500" />
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardContent className="pt-6">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm text-muted-foreground">Unreconciled Items</p>
                <p className="text-xl font-bold">34</p>
              </div>
              <AlertTriangle className="h-8 w-8 text-rose-500" />
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Quick actions hint */}
      <Card className="border-dashed">
        <CardContent className="flex items-center justify-center py-8 text-sm text-muted-foreground">
          Full CFO dashboard with charts and analytics is under development.
          Revenue trends, cash flow, and compliance timeline coming soon.
        </CardContent>
      </Card>
    </div>
  );
}

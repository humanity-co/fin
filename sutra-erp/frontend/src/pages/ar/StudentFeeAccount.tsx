import { useMemo } from "react";
import { Link, useParams } from "react-router";
import { CreditCard, Download, ReceiptText, UserRound } from "lucide-react";
import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button, TableSkeleton } from "../../components/ui";
import { MoneyDisplay, StatusBadge } from "../../components/data";
import { useMockQuery } from "../../hooks/useMockQuery";
import { MOCK_STUDENTS, MOCK_STUDENT_FEE_ACCOUNTS, MOCK_FEE_STRUCTURES, type StudentFeeAccount } from "../../lib/mock-data";
import { formatIndianDate } from "../../lib/formatters";

/**
 * Student Fee Account — gross fees → concessions → scholarships → net
 * payable → paid → balance, with installment schedule and payment history.
 */
export default function StudentFeeAccountPage() {
  const { studentId } = useParams<{ studentId: string }>();
  const { data: account, isLoading } = useMockQuery<StudentFeeAccount | undefined>(
    MOCK_STUDENT_FEE_ACCOUNTS[studentId ?? "stu-001"]
  );
  const student = MOCK_STUDENTS.find((s) => s.student_id === studentId) ?? MOCK_STUDENTS[0];
  const structure = MOCK_FEE_STRUCTURES.find((f) => f.fee_structure_id === account?.fee_structure_id);

  const breakdown = useMemo(() => {
    if (!account) return { gross: 0, concession: 0, scholarship: 0, net: 0, paid: 0, balance: 0, pct: 0 };
    const pct = account.gross_fees > 0 ? Math.round((account.paid / account.net_payable) * 100) : 0;
    return { gross: account.gross_fees, concession: account.concession, scholarship: account.scholarship, net: account.net_payable, paid: account.paid, balance: account.balance, pct };
  }, [account]);

  if (isLoading) {
    return (
      <div className="animate-in fade-in duration-500 space-y-6">
        <PageHeader title="Student Fee Account" description="Loading student fee details..." breadcrumbs={[{ label: "Accounts Receivable" }, { label: "Students" }, { label: "Fee Account" }]} />
        <Card className="glass border-white/60 p-6"><TableSkeleton rows={8} cols={4} /></Card>
      </div>
    );
  }

  if (!account) {
    return (
      <div className="animate-in fade-in duration-500">
        <PageHeader title="Student Fee Account" description="Fee account not found" breadcrumbs={[{ label: "Accounts Receivable" }, { label: "Students" }]} />
        <Card className="glass border-white/60 p-12 text-center">
          <p className="text-sm font-semibold text-slate-600">No fee account exists for this student.</p>
          <Link to="/ar/payments/receipts" className="mt-2 inline-block text-xs font-bold text-primary hover:underline">← Back to Fee Collection</Link>
        </Card>
      </div>
    );
  }

  return (
    <div className="animate-in fade-in duration-500 space-y-6">
      <PageHeader
        title={`${student.name} — Fee Account`}
        description={`${student.enrollment} · ${student.program} · ${structure?.name ?? "No structure"}`}
        breadcrumbs={[
          { label: "Accounts Receivable", href: "/ar/fee-structures" },
          { label: "Fee Collection", href: "/ar/payments/receipts" },
          { label: student.name },
        ]}
        actions={
          <div className="flex gap-2">
            <Button variant="outline" size="sm" className="glass-input hover-lift">
              <Download className="h-4 w-4 mr-2" /> Download Statement
            </Button>
            <Link to="/ar/payments/receipts">
              <Button size="sm" className="bg-primary hover:bg-primary/90 text-white shadow-lg hover-lift">
                <CreditCard className="h-4 w-4 mr-2" /> Collect Payment
              </Button>
            </Link>
          </div>
        }
      />

      {/* Student identity strip */}
      <Card className="glass border-white/60 p-4 shadow-sm">
        <div className="flex flex-wrap items-center gap-6 text-xs text-slate-600">
          <div className="flex items-center gap-2">
            <div className="flex h-10 w-10 items-center justify-center rounded-full bg-gradient-to-tr from-primary to-indigo-400 text-white">
              <UserRound className="h-5 w-5" />
            </div>
            <div>
              <p className="font-bold text-slate-800">{student.name}</p>
              <p className="font-mono text-[10px] text-slate-400">{student.enrollment}</p>
            </div>
          </div>
          <div><p className="font-bold uppercase tracking-wider text-slate-400 text-[9px]">Category</p><p className="font-semibold">{student.category}</p></div>
          <div><p className="font-bold uppercase tracking-wider text-slate-400 text-[9px]">Batch</p><p className="font-semibold">{student.batch}</p></div>
          <div><p className="font-bold uppercase tracking-wider text-slate-400 text-[9px]">Hostel</p><p className="font-semibold">{student.hostel ? "Resident" : "Day Scholar"}</p></div>
          <div><p className="font-bold uppercase tracking-wider text-slate-400 text-[9px]">Father</p><p className="font-semibold">{student.father_name}</p></div>
          <div><p className="font-bold uppercase tracking-wider text-slate-400 text-[9px]">FRC Approval</p><p className="font-mono text-[11px] font-semibold">{structure?.frc_approval}</p></div>
          <div className="ml-auto"><StatusBadge status={breakdown.balance > 0 ? "PartiallyPaid" : "Paid"} variant={breakdown.balance > 0 ? "warning" : "success"} /></div>
        </div>
      </Card>

      {/* Fee breakdown */}
      <div className="grid grid-cols-2 md:grid-cols-5 gap-4">
        <BreakdownCard label="Gross Fees" amount={breakdown.gross} tone="text-slate-800" />
        <BreakdownCard label="Concession" amount={breakdown.concession} tone="text-amber-600" negative />
        <BreakdownCard label="Scholarship" amount={breakdown.scholarship} tone="text-sky-700" negative />
        <BreakdownCard label="Net Payable" amount={breakdown.net} tone="text-primary" strong />
        <BreakdownCard label="Balance Due" amount={breakdown.balance} tone={breakdown.balance > 0 ? "text-rose-600" : "text-emerald-700"} strong />
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Installment schedule */}
        <Card className="glass border-white/60 shadow-lg">
          <div className="border-b border-white/40 bg-white/40 p-4 text-sm font-bold text-slate-800">
            Installment Schedule · {structure?.installment_plans[0]?.name ?? "2 Installments"}
          </div>
          <CardContent className="p-0">
            <div className="divide-y divide-white/40">
              {account.installment_schedule.map((inst) => (
                <div key={inst.number} className="flex items-center justify-between px-4 py-3">
                  <div>
                    <p className="text-xs font-bold text-slate-700">{inst.label}</p>
                    <p className="text-[10px] font-mono text-slate-400">Due {formatIndianDate(inst.due_date)}</p>
                  </div>
                  <div className="text-right">
                    <p className="text-xs font-semibold text-slate-600"><MoneyDisplay amount={inst.amount} /></p>
                    <p className="text-[10px] text-slate-400">Paid <MoneyDisplay amount={inst.paid} /></p>
                  </div>
                  <StatusBadge status={inst.status} />
                </div>
              ))}
            </div>
            {/* Collection progress */}
            <div className="border-t border-white/40 bg-white/50 p-4">
              <div className="mb-1 flex justify-between text-[10px] font-bold uppercase tracking-wider text-slate-500">
                <span>Collection Progress</span>
                <span>{breakdown.pct}%</span>
              </div>
              <div className="h-2.5 w-full overflow-hidden rounded-full bg-slate-200/70">
                <div className="h-full rounded-full bg-gradient-to-r from-emerald-500 to-emerald-400 transition-all" style={{ width: `${breakdown.pct}%` }} />
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Payment history */}
        <Card className="glass border-white/60 shadow-lg">
          <div className="border-b border-white/40 bg-white/40 p-4 text-sm font-bold text-slate-800">Payment History ({account.payments.length})</div>
          <CardContent className="p-0">
            {account.payments.length === 0 ? (
              <div className="flex flex-col items-center justify-center gap-2 py-10 text-center">
                <ReceiptText className="h-8 w-8 text-slate-300" />
                <p className="text-xs font-semibold text-slate-600">No payments recorded yet</p>
                <p className="text-xs text-slate-400">Collect the first installment from the Fee Collection screen.</p>
              </div>
            ) : (
              <div className="divide-y divide-white/40">
                {account.payments.map((p) => (
                  <div key={p.receipt_no} className="flex items-center justify-between px-4 py-3">
                    <div>
                      <p className="font-mono text-xs font-bold text-primary">{p.receipt_no}</p>
                      <p className="text-[10px] font-mono text-slate-400">{formatIndianDate(p.date)} · {p.head}</p>
                    </div>
                    <div className="text-right">
                      <p className="text-xs font-extrabold text-slate-800"><MoneyDisplay amount={p.amount} /></p>
                      <p className="text-[10px] text-slate-400">{p.mode}{p.reference ? ` · ${p.reference}` : ""}</p>
                    </div>
                    <StatusBadge status={p.status} />
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

function BreakdownCard({ label, amount, tone, negative, strong }: { label: string; amount: number; tone: string; negative?: boolean; strong?: boolean }) {
  return (
    <Card className="glass border-white/60 p-4 shadow-sm">
      <p className="text-[10px] font-bold uppercase tracking-widest text-slate-500">{label}</p>
      <p className={`mt-1 ${strong ? "text-lg" : "text-base"} font-extrabold ${tone}`}>
        {negative && amount > 0 ? "−" : ""}<MoneyDisplay amount={amount} />
      </p>
    </Card>
  );
}

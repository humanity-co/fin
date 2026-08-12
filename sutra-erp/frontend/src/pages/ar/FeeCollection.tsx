import { useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { ArrowRight, CheckCircle2, Receipt, Search, UserRound } from "lucide-react";
import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button, Input } from "../../components/ui";
import { MoneyDisplay, StatusBadge } from "../../components/data";
import { MOCK_STUDENTS, MOCK_STUDENT_FEE_ACCOUNTS, findStudent, type Student } from "../../lib/mock-data";
import { formatIndianDate } from "../../lib/formatters";

const PAYMENT_MODES = ["UPI", "NEFT", "RTGS", "Cash", "Cheque", "Demand Draft"];

/**
 * Fee Collection — search student → view assessment → record payment
 * (mode, reference, date) → receipt confirmation. Links to full fee account.
 */
export default function FeeCollection() {
  const navigate = useNavigate();
  const [query, setQuery] = useState("");
  const [student, setStudent] = useState<Student | null>(null);
  const [showResults, setShowResults] = useState(false);
  const [mode, setMode] = useState("UPI");
  const [amount, setAmount] = useState("");
  const [reference, setReference] = useState("");
  const [payDate, setPayDate] = useState(() => new Date().toISOString().split("T")[0]);
  const [receipt, setReceipt] = useState<{ no: string; amount: number } | null>(null);

  const account = student ? MOCK_STUDENT_FEE_ACCOUNTS[student.student_id] : undefined;

  const results = useMemo(() => {
    if (!query.trim()) return [];
    const q = query.toLowerCase();
    return MOCK_STUDENTS.filter(
      (s) => s.name.toLowerCase().includes(q) || s.enrollment.toLowerCase().includes(q)
    ).slice(0, 6);
  }, [query]);

  const selectStudent = (s: Student) => {
    setStudent(s);
    setQuery(`${s.name} (${s.enrollment})`);
    setShowResults(false);
    setReceipt(null);
    const bal = MOCK_STUDENT_FEE_ACCOUNTS[s.student_id]?.balance ?? 0;
    if (bal > 0) setAmount(String(bal / 100));
  };

  const suggestedAmount = account ? account.balance : 0;
  const entered = Math.round((Number(amount) || 0) * 100);
  const overpay = entered > suggestedAmount && suggestedAmount > 0;

  const handleRecord = () => {
    if (!student) { alert("Search and select a student first."); return; }
    if (!amount || Number(amount) <= 0) { alert("Enter a valid payment amount."); return; }
    setReceipt({ no: `RCPT-2026-${String(420 + Math.floor(Math.random() * 50)).padStart(4, "0")}`, amount: entered });
  };

  const reset = () => {
    setStudent(null); setQuery(""); setAmount(""); setReference(""); setReceipt(null);
  };

  return (
    <div className="animate-in fade-in duration-500 space-y-6">
      <PageHeader
        title="Fee Collection"
        description="Search a student, review the assessment and record a payment"
        breadcrumbs={[{ label: "Accounts Receivable" }, { label: "Fee Collection" }]}
        actions={
          receipt ? (
            <Button size="sm" onClick={reset} className="bg-slate-800 hover:bg-slate-700 text-white shadow-lg hover-lift">
              New Collection
            </Button>
          ) : (
            <Button size="sm" onClick={handleRecord} disabled={!student || !amount} className="bg-emerald-600 hover:bg-emerald-500 text-white shadow-[0_0_15px_rgba(16,185,129,0.4)] hover-lift disabled:opacity-40">
              <Receipt className="h-4 w-4 mr-2" /> Generate Receipt
            </Button>
          )
        }
      />

      {receipt ? (
        <Card className="glass border-emerald-200 shadow-lg overflow-hidden">
          <div className="bg-gradient-to-r from-emerald-600 to-emerald-500 p-6 text-center text-white">
            <CheckCircle2 className="mx-auto mb-2 h-12 w-12" />
            <h2 className="text-xl font-extrabold">Payment Recorded</h2>
            <p className="text-sm opacity-90">Receipt generated successfully</p>
          </div>
          <CardContent className="p-6 text-sm">
            <div className="mx-auto max-w-md space-y-2">
              <Row k="Receipt No" v={<span className="font-mono font-bold text-primary">{receipt.no}</span>} />
              <Row k="Student" v={`${student?.name} (${student?.enrollment})`} />
              <Row k="Amount" v={<span className="font-extrabold text-slate-800"><MoneyDisplay amount={receipt.amount} /></span>} />
              <Row k="Mode" v={mode} />
              <Row k="Date" v={formatIndianDate(payDate)} />
              <Row k="Balance After" v={<MoneyDisplay amount={Math.max(0, suggestedAmount - receipt.amount)} />} />
            </div>
            <div className="mt-6 flex justify-center gap-3">
              <Button variant="outline" onClick={reset}>Record Another</Button>
              {student && (
                <Button className="bg-primary text-white" onClick={() => navigate(`/ar/students/${student.student_id}/fees`)}>
                  Open Full Fee Account <ArrowRight className="h-4 w-4 ml-1" />
                </Button>
              )}
            </div>
          </CardContent>
        </Card>
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-5 gap-6">
          {/* Left: search + assessment */}
          <div className="lg:col-span-3 space-y-6">
            <Card className="glass border-white/60 shadow-lg">
              <div className="border-b border-white/40 bg-white/40 p-4 text-sm font-bold text-slate-800">1 · Find Student</div>
              <CardContent className="p-4">
                <div className="relative">
                  <Search className="absolute left-3 top-3 h-5 w-5 text-slate-400" />
                  <Input
                    placeholder="Search by name or enrollment no. (e.g. SRCOE2026CSE001)"
                    className="pl-10 h-12 text-base glass-input shadow-inner"
                    value={query}
                    onChange={(e) => { setQuery(e.target.value); setShowResults(true); setStudent(null); setReceipt(null); }}
                    onFocus={() => setShowResults(true)}
                    onBlur={() => setTimeout(() => setShowResults(false), 150)}
                  />
                  {showResults && results.length > 0 && (
                    <div className="absolute z-20 mt-1 w-full overflow-hidden rounded-xl border border-white/60 bg-white/95 shadow-2xl backdrop-blur-md">
                      {results.map((s) => {
                        const bal = MOCK_STUDENT_FEE_ACCOUNTS[s.student_id]?.balance ?? 0;
                        return (
                          <button
                            key={s.student_id}
                            onMouseDown={() => selectStudent(s)}
                            className="flex w-full items-center justify-between gap-3 px-4 py-2.5 text-left hover:bg-primary/5 transition-colors"
                          >
                            <div className="flex items-center gap-3">
                              <UserRound className="h-4 w-4 text-slate-400" />
                              <div>
                                <p className="text-sm font-semibold text-slate-800">{s.name}</p>
                                <p className="font-mono text-[10px] text-slate-400">{s.enrollment} · {s.program}</p>
                              </div>
                            </div>
                            {bal > 0 ? (
                              <span className="text-xs font-bold text-rose-600">Due <MoneyDisplay amount={bal} /></span>
                            ) : (
                              <StatusBadge status="Paid" />
                            )}
                          </button>
                        );
                      })}
                    </div>
                  )}
                </div>

                {student && (
                  <div className="mt-4 flex items-center justify-between rounded-xl border border-white/50 bg-white/50 p-4">
                    <div>
                      <p className="text-sm font-bold text-slate-800">{student.name}</p>
                      <p className="text-xs text-slate-500">{student.program} · {student.batch} · {student.category}</p>
                    </div>
                    <div className="text-right">
                      <p className="text-[10px] font-bold uppercase tracking-wider text-slate-400">Net Payable</p>
                      <p className="text-base font-extrabold text-slate-800"><MoneyDisplay amount={account?.net_payable ?? 0} /></p>
                    </div>
                  </div>
                )}
              </CardContent>
            </Card>

            {/* Assessment */}
            <Card className="glass border-white/60 shadow-lg">
              <div className="border-b border-white/40 bg-white/40 p-4 text-sm font-bold text-slate-800">2 · Fee Assessment</div>
              <CardContent className="p-0">
                {!student ? (
                  <div className="p-10 text-center text-xs text-slate-400">Select a student to view the fee assessment.</div>
                ) : (
                  <div className="divide-y divide-white/40">
                    <AssessRow label="Gross Fees (annual)" value={account?.gross_fees ?? 0} />
                    <AssessRow label="Concession" value={-(account?.concession ?? 0)} tone="text-amber-600" />
                    <AssessRow label="Scholarship adjustment" value={-(account?.scholarship ?? 0)} tone="text-sky-700" />
                    <AssessRow label="Net Payable" value={account?.net_payable ?? 0} bold />
                    <AssessRow label="Total Paid" value={account?.paid ?? 0} tone="text-emerald-700" />
                    <div className="flex items-center justify-between bg-rose-50/60 px-4 py-3">
                      <span className="text-xs font-extrabold uppercase tracking-wider text-rose-700">Balance Due</span>
                      <span className="text-base font-extrabold text-rose-700"><MoneyDisplay amount={account?.balance ?? 0} /></span>
                    </div>
                    {student && (
                      <div className="flex justify-end px-4 py-2">
                        <button className="text-[11px] font-bold text-primary hover:underline" onClick={() => navigate(`/ar/students/${student.student_id}/fees`)}>
                          View full fee account with installment schedule →
                        </button>
                      </div>
                    )}
                  </div>
                )}
              </CardContent>
            </Card>
          </div>

          {/* Right: payment form */}
          <div className="lg:col-span-2">
            <Card className="glass border-white/60 shadow-lg">
              <div className="border-b border-white/40 bg-white/40 p-4 text-sm font-bold text-slate-800">3 · Payment Details</div>
              <CardContent className="p-5 space-y-4 text-sm">
                <div>
                  <Label>Amount (₹) {suggestedAmount > 0 && <button className="ml-1 text-[10px] font-bold text-primary hover:underline" onClick={() => setAmount(String(suggestedAmount / 100))}>Use balance due</button>}</Label>
                  <Input type="number" value={amount} onChange={(e) => setAmount(e.target.value)} placeholder="0.00" className="h-12 text-xl font-extrabold font-mono text-emerald-700 glass-input" />
                  {overpay && <p className="mt-1 text-[10px] font-bold text-amber-600">Amount exceeds balance due — excess will be held as advance.</p>}
                </div>
                <div>
                  <Label>Payment Mode</Label>
                  <div className="grid grid-cols-3 gap-1.5">
                    {PAYMENT_MODES.map((m) => (
                      <button
                        key={m}
                        onClick={() => setMode(m)}
                        className={`rounded-lg border px-2 py-2 text-[11px] font-bold transition-all ${
                          mode === m
                            ? "border-primary bg-primary/10 text-primary shadow-sm"
                            : "border-white/60 bg-white/50 text-slate-500 hover:border-primary/40"
                        }`}
                      >
                        {m}
                      </button>
                    ))}
                  </div>
                </div>
                {mode !== "Cash" && (
                  <div>
                    <Label>Reference / UTR / Cheque No.</Label>
                    <Input value={reference} onChange={(e) => setReference(e.target.value)} placeholder={mode === "UPI" ? "UPI transaction ID" : mode === "Cheque" ? "Cheque number & bank" : "UTR / NEFT reference"} className="glass-input" />
                  </div>
                )}
                <div>
                  <Label>Payment Date</Label>
                  <Input type="date" value={payDate} onChange={(e) => setPayDate(e.target.value)} className="glass-input" />
                </div>

                <div className="rounded-xl border border-emerald-200 bg-emerald-50/70 p-4">
                  <div className="flex justify-between text-xs font-semibold text-slate-600">
                    <span>Amount to collect</span>
                    <span className="text-base font-extrabold text-emerald-700"><MoneyDisplay amount={entered} /></span>
                  </div>
                  {student && account && (
                    <p className="mt-1 text-[10px] text-slate-400">
                      Balance after this payment: <MoneyDisplay amount={Math.max(0, account.balance - entered)} />
                    </p>
                  )}
                </div>

                <Button
                  onClick={handleRecord}
                  disabled={!student || !amount || Number(amount) <= 0}
                  className="w-full bg-emerald-600 hover:bg-emerald-500 text-white h-12 shadow-[0_0_15px_rgba(16,185,129,0.35)] hover-lift disabled:opacity-40"
                >
                  <Receipt className="h-4 w-4 mr-2" /> Record Payment & Print Receipt
                </Button>
                <p className="text-center text-[10px] text-slate-400">The receipt will be posted to the student's fee account ledger.</p>
              </CardContent>
            </Card>
          </div>
        </div>
      )}
    </div>
  );
}

function AssessRow({ label, value, tone = "text-slate-700", bold }: { label: string; value: number; tone?: string; bold?: boolean }) {
  return (
    <div className="flex items-center justify-between px-4 py-2.5">
      <span className="text-xs font-semibold text-slate-500">{label}</span>
      <span className={`${bold ? "text-sm font-extrabold text-slate-900" : "text-xs font-bold"} ${tone}`}>
        {value < 0 && "−"}<MoneyDisplay amount={Math.abs(value)} />
      </span>
    </div>
  );
}

function Row({ k, v }: { k: string; v: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between border-b border-dashed border-white/40 pb-2">
      <span className="text-xs font-semibold text-slate-500">{k}</span>
      <span className="text-sm font-semibold text-slate-700">{v}</span>
    </div>
  );
}

function Label({ children }: { children: React.ReactNode }) {
  return <label className="mb-1 block text-[10px] font-bold uppercase tracking-wider text-slate-500">{children}</label>;
}

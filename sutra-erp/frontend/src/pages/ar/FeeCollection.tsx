import { useState } from "react";
import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button } from "../../components/ui";
import { Save, Search, Receipt } from "lucide-react";

export default function FeeCollection() {
  const [studentId, setStudentId] = useState("");
  const [amount, setAmount] = useState("");
  const [paymentMode, setPaymentMode] = useState("Bank Transfer");
  const [reference, setReference] = useState("");
  const [receiptGenerated, setReceiptGenerated] = useState(false);

  const handleSave = () => {
    if (!studentId || !amount) {
      alert("Please enter Student ID and Amount");
      return;
    }
    // Simulate API call
    setTimeout(() => {
      setReceiptGenerated(true);
      alert("Fee Receipt Generated Successfully!");
    }, 500);
  };

  const reset = () => {
    setStudentId("");
    setAmount("");
    setReference("");
    setReceiptGenerated(false);
  };

  return (
    <div className="animate-in fade-in duration-500 h-full flex flex-col">
      <PageHeader
        title="Fee Collection Point of Sale"
        description="Quickly record student payments and generate receipts"
        breadcrumbs={[{ label: "Accounts Receivable" }, { label: "Fee Collection" }]}
        actions={
          receiptGenerated ? (
            <Button size="sm" onClick={reset} className="bg-slate-800 hover:bg-slate-700 text-white shadow-lg hover-lift">
              New Collection
            </Button>
          ) : (
            <Button size="sm" onClick={handleSave} className="bg-emerald-600 hover:bg-emerald-500 text-white shadow-[0_0_15px_rgba(16,185,129,0.4)] hover-lift">
              <Save className="h-4 w-4 mr-2" /> Generate Receipt
            </Button>
          )
        }
      />

      <div className="grid grid-cols-1 md:grid-cols-3 gap-6 flex-1">
        {/* Collection Form */}
        <div className="md:col-span-2 flex flex-col gap-6">
          <Card className="glass border-white/60 shadow-lg relative overflow-hidden">
            <div className="absolute top-0 right-0 p-4 opacity-10 pointer-events-none">
              <Receipt className="w-48 h-48" />
            </div>
            <div className="p-6 relative z-10 border-b border-white/40">
              <h2 className="text-lg font-bold text-slate-800">Payment Details</h2>
            </div>
            <CardContent className="p-6 relative z-10 grid grid-cols-2 gap-6">
              <div className="col-span-2">
                <label className="block text-xs font-bold uppercase tracking-wider text-slate-500 mb-2">Student ID / Enrollment No.</label>
                <div className="relative">
                  <Search className="absolute left-3 top-2.5 h-5 w-5 text-slate-400" />
                  <input 
                    type="text" 
                    value={studentId}
                    onChange={(e) => setStudentId(e.target.value)}
                    placeholder="Search Student..." 
                    className="w-full pl-10 pr-4 py-3 text-lg font-semibold glass-input rounded-xl shadow-inner focus:outline-none focus:ring-2 focus:ring-emerald-500/50 transition-all"
                  />
                </div>
              </div>

              <div>
                <label className="block text-xs font-bold uppercase tracking-wider text-slate-500 mb-2">Amount (₹)</label>
                <input 
                  type="number" 
                  value={amount}
                  onChange={(e) => setAmount(e.target.value)}
                  placeholder="0.00" 
                  className="w-full px-4 py-3 text-2xl font-mono font-extrabold text-emerald-700 bg-emerald-50/50 border border-emerald-200 rounded-xl shadow-inner focus:outline-none focus:ring-2 focus:ring-emerald-500/50"
                />
              </div>

              <div>
                <label className="block text-xs font-bold uppercase tracking-wider text-slate-500 mb-2">Payment Mode</label>
                <select 
                  value={paymentMode}
                  onChange={(e) => setPaymentMode(e.target.value)}
                  className="w-full px-4 py-3 text-lg font-semibold glass-input rounded-xl shadow-inner focus:outline-none focus:ring-2 focus:ring-emerald-500/50"
                >
                  <option>Bank Transfer</option>
                  <option>UPI</option>
                  <option>Cash</option>
                  <option>Cheque</option>
                </select>
              </div>

              <div className="col-span-2">
                <label className="block text-xs font-bold uppercase tracking-wider text-slate-500 mb-2">Reference Number / UTR</label>
                <input 
                  type="text" 
                  value={reference}
                  onChange={(e) => setReference(e.target.value)}
                  placeholder="Txn ID or Cheque No." 
                  className="w-full px-4 py-3 text-sm font-medium glass-input rounded-xl shadow-inner focus:outline-none focus:ring-2 focus:ring-emerald-500/50"
                />
              </div>
            </CardContent>
          </Card>
        </div>

        {/* Student Ledger Summary */}
        <div className="flex flex-col gap-6">
          <Card className="glass border-white/60 shadow-lg flex-1">
            <div className="p-6 border-b border-white/40">
              <h2 className="text-lg font-bold text-slate-800">Student Outstanding</h2>
            </div>
            <CardContent className="p-6">
              {studentId ? (
                <div className="space-y-6">
                  <div>
                    <h3 className="text-sm font-bold text-slate-800">John Doe (B.Tech CS)</h3>
                    <p className="text-xs font-medium text-slate-500">Year 2, Semester 3</p>
                  </div>
                  
                  <div className="bg-rose-50/50 border border-rose-100 rounded-lg p-4">
                    <p className="text-[10px] font-bold uppercase tracking-widest text-rose-500 mb-1">Total Due</p>
                    <p className="font-mono text-3xl font-extrabold text-rose-700">₹ 85,000.00</p>
                  </div>

                  <div className="space-y-3">
                    <div className="flex justify-between items-center text-sm border-b border-white/40 pb-2">
                      <span className="font-medium text-slate-600">Tuition Fee</span>
                      <span className="font-mono font-bold text-slate-800">₹ 70,000</span>
                    </div>
                    <div className="flex justify-between items-center text-sm border-b border-white/40 pb-2">
                      <span className="font-medium text-slate-600">Hostel Fee</span>
                      <span className="font-mono font-bold text-slate-800">₹ 12,000</span>
                    </div>
                    <div className="flex justify-between items-center text-sm border-b border-white/40 pb-2">
                      <span className="font-medium text-slate-600">Exam Fee</span>
                      <span className="font-mono font-bold text-slate-800">₹ 3,000</span>
                    </div>
                  </div>
                </div>
              ) : (
                <div className="h-full flex flex-col items-center justify-center text-slate-400 py-12">
                  <Search className="w-12 h-12 mb-4 opacity-20" />
                  <p className="text-sm font-medium">Search a student to view outstanding fees</p>
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}

import { 
  TrendingUp, ArrowUpRight, ArrowDownRight, CheckCircle, Clock 
} from "lucide-react";
import { AreaChart, Area, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts';

const revenueData = [
  { name: 'Apr', value: 1200000 },
  { name: 'May', value: 1350000 },
  { name: 'Jun', value: 1100000 },
  { name: 'Jul', value: 1650000 },
  { name: 'Aug', value: 1400000 },
  { name: 'Sep', value: 1800000 },
];

const RECENT_TRANSACTIONS = [
  { id: "JRN-24-001", date: "2026-07-31", ledger: "Salary A/C", type: "Payment", amount: 450000, status: "Posted" },
  { id: "RCP-24-089", date: "2026-07-31", ledger: "Student Fees", type: "Receipt", amount: 125000, status: "Cleared" },
  { id: "JRN-24-002", date: "2026-07-30", ledger: "Office Rent", type: "Payment", amount: 80000, status: "Posted" },
  { id: "INV-24-045", date: "2026-07-29", ledger: "IT Hardware", type: "Purchase", amount: 350000, status: "Pending" },
  { id: "RCP-24-090", date: "2026-07-29", ledger: "Consultancy Income", type: "Receipt", amount: 75000, status: "Cleared" },
];

export default function CfoDashboard() {
  return (
    <div className="flex flex-col gap-6">
      <header>
        <h1 className="text-xl font-bold text-slate-800">Financial Overview</h1>
        <p className="text-xs text-slate-500">Gateway &gt; Dashboard &gt; FY 2026-27</p>
      </header>

      {/* KPI Section */}
      <div className="grid grid-cols-4 gap-4">
        {[
          { label: "Net Cash Flow", value: "₹ 45,23,000", trend: "+12%", up: true, icon: TrendingUp },
          { label: "Total Receivables", value: "₹ 12,50,000", trend: "-5%", up: false, icon: ArrowDownRight },
          { label: "Total Payables", value: "₹ 8,30,000", trend: "+2%", up: false, icon: ArrowUpRight },
          { label: "Working Capital", value: "₹ 34,43,000", trend: "+18%", up: true, icon: CheckCircle },
        ].map((kpi, idx) => (
          <div key={idx} className="bg-white border border-slate-200 shadow-sm rounded-md p-4 flex flex-col justify-between relative overflow-hidden group hover:border-indigo-300 transition-colors">
            <div className="flex justify-between items-start mb-4">
              <span className="text-[11px] font-bold uppercase tracking-wider text-slate-500">{kpi.label}</span>
              <kpi.icon className={`h-4 w-4 ${kpi.up ? 'text-emerald-500' : 'text-rose-500'}`} />
            </div>
            <div className="text-2xl font-bold text-slate-800 font-mono tracking-tight">{kpi.value}</div>
            <div className={`text-[10px] font-semibold mt-1 ${kpi.up ? 'text-emerald-600' : 'text-rose-600'}`}>
              {kpi.trend} vs Last Month
            </div>
            <div className="absolute -right-4 -bottom-4 opacity-5 group-hover:opacity-10 transition-opacity">
              <kpi.icon className="h-24 w-24" />
            </div>
          </div>
        ))}
      </div>

      <div className="grid grid-cols-3 gap-6">
        {/* Chart Section */}
        <div className="col-span-2 bg-white border border-slate-200 shadow-sm rounded-md flex flex-col">
          <div className="border-b border-slate-100 p-4">
            <h3 className="text-xs font-bold uppercase tracking-wider text-slate-600">Revenue Trend (YTD)</h3>
          </div>
          <div className="p-4 flex-1 min-h-[250px]">
            <ResponsiveContainer width="100%" height="100%">
              <AreaChart data={revenueData}>
                <defs>
                  <linearGradient id="colorValue" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#4f46e5" stopOpacity={0.3}/>
                    <stop offset="95%" stopColor="#4f46e5" stopOpacity={0}/>
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" vertical={false} stroke="#e2e8f0" />
                <XAxis dataKey="name" axisLine={false} tickLine={false} tick={{ fontSize: 11, fill: '#64748b' }} dy={10} />
                <YAxis axisLine={false} tickLine={false} tick={{ fontSize: 11, fill: '#64748b' }} tickFormatter={(val) => `₹${val/100000}L`} dx={-10} />
                <Tooltip 
                  contentStyle={{ backgroundColor: '#1e293b', border: 'none', borderRadius: '4px', fontSize: '12px', color: '#f8fafc' }}
                  itemStyle={{ color: '#818cf8' }}
                  formatter={(val: any) => [`₹ ${Number(val).toLocaleString('en-IN')}`, 'Revenue']}
                />
                <Area type="monotone" dataKey="value" stroke="#4f46e5" strokeWidth={2} fillOpacity={1} fill="url(#colorValue)" />
              </AreaChart>
            </ResponsiveContainer>
          </div>
        </div>

        {/* Quick Actions & Approvals */}
        <div className="flex flex-col gap-4">
          <div className="bg-slate-900 border border-slate-800 shadow-sm rounded-md p-4 text-white">
            <h3 className="text-xs font-bold uppercase tracking-wider text-slate-400 mb-3">Gateway Quick Actions</h3>
            <div className="grid grid-cols-2 gap-2">
              <button className="bg-slate-800 hover:bg-indigo-600 transition-colors text-xs font-medium py-2 rounded border border-slate-700 hover:border-indigo-500">
                New Voucher <span className="block text-[9px] text-slate-400 mt-0.5">Alt + V</span>
              </button>
              <button className="bg-slate-800 hover:bg-indigo-600 transition-colors text-xs font-medium py-2 rounded border border-slate-700 hover:border-indigo-500">
                Day Book <span className="block text-[9px] text-slate-400 mt-0.5">Alt + D</span>
              </button>
              <button className="bg-slate-800 hover:bg-indigo-600 transition-colors text-xs font-medium py-2 rounded border border-slate-700 hover:border-indigo-500">
                Trial Balance <span className="block text-[9px] text-slate-400 mt-0.5">Alt + T</span>
              </button>
              <button className="bg-slate-800 hover:bg-indigo-600 transition-colors text-xs font-medium py-2 rounded border border-slate-700 hover:border-indigo-500">
                Ledgers <span className="block text-[9px] text-slate-400 mt-0.5">Alt + L</span>
              </button>
            </div>
          </div>

          <div className="bg-amber-50 border border-amber-200 shadow-sm rounded-md p-4 flex-1 flex flex-col justify-center items-center text-center">
            <div className="h-10 w-10 bg-amber-100 rounded-full flex items-center justify-center mb-2">
              <Clock className="h-5 w-5 text-amber-600" />
            </div>
            <h4 className="text-sm font-bold text-amber-900">14 Pending Approvals</h4>
            <p className="text-xs text-amber-700 mt-1">Purchase Invoices & Journal Vouchers awaiting CFO authorization.</p>
            <button className="mt-3 bg-amber-600 text-white text-xs font-bold px-4 py-1.5 rounded hover:bg-amber-700 transition-colors shadow-sm">
              Review Now
            </button>
          </div>
        </div>
      </div>

      {/* Dense Data Table */}
      <div className="bg-white border border-slate-200 shadow-sm rounded-md overflow-hidden flex flex-col">
        <div className="border-b border-slate-100 p-4 flex justify-between items-center bg-slate-50">
          <h3 className="text-xs font-bold uppercase tracking-wider text-slate-600">Recent Ledger Activity</h3>
          <button className="text-xs font-semibold text-indigo-600 hover:text-indigo-800">View Day Book &rarr;</button>
        </div>
        <table className="w-full text-left border-collapse">
          <thead>
            <tr className="bg-slate-100 border-b border-slate-200 text-[10px] font-bold uppercase tracking-wider text-slate-500">
              <th className="p-3">Date</th>
              <th className="p-3">Voucher Ref</th>
              <th className="p-3">Primary Ledger</th>
              <th className="p-3">Type</th>
              <th className="p-3 text-right">Amount (₹)</th>
              <th className="p-3 text-right">Status</th>
            </tr>
          </thead>
          <tbody>
            {RECENT_TRANSACTIONS.map((tx, idx) => (
              <tr key={idx} className="border-b border-slate-100 hover:bg-slate-50 transition-colors text-xs font-medium text-slate-700">
                <td className="p-3">{tx.date}</td>
                <td className="p-3 font-mono text-indigo-600">{tx.id}</td>
                <td className="p-3 font-bold">{tx.ledger}</td>
                <td className="p-3">{tx.type}</td>
                <td className="p-3 text-right font-mono font-bold">{tx.amount.toLocaleString('en-IN', { minimumFractionDigits: 2 })}</td>
                <td className="p-3 text-right">
                  <span className={`px-2 py-0.5 rounded-full text-[10px] font-bold uppercase tracking-wider ${
                    tx.status === 'Posted' || tx.status === 'Cleared' 
                      ? 'bg-emerald-100 text-emerald-700' 
                      : 'bg-amber-100 text-amber-700'
                  }`}>
                    {tx.status}
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

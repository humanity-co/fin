import { useState } from "react";
import { Building2, CalendarDays, Landmark, Percent, Save, Settings2 } from "lucide-react";
import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button } from "../../components/ui";

export default function SystemSettings() {
  const [institution, setInstitution] = useState("Shri Ram College of Engineering, Nashik");
  const [gstin, setGstin] = useState("27AABCS1234F1Z5");
  const [pan, setPan] = useState("AABCS1234F");
  const [fy, setFy] = useState("2026-27");
  const [saved, setSaved] = useState(false);

  const handleSave = () => {
    setSaved(true);
    setTimeout(() => setSaved(false), 2500);
  };

  return (
    <div className="animate-in fade-in duration-500 space-y-6">
      <PageHeader
        title="System Settings"
        description="Institution profile, fiscal year and tax configuration"
        breadcrumbs={[{ label: "System" }, { label: "Settings" }]}
        actions={<Button size="sm" onClick={handleSave} className="bg-primary hover:bg-primary/90 text-white shadow-lg hover-lift"><Save className="h-4 w-4 mr-1" /> Save Changes</Button>}
      />
      {saved && <div className="rounded-xl border border-emerald-200 bg-emerald-50/80 px-4 py-3 text-xs font-bold text-emerald-700">Settings saved successfully.</div>}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <Card className="glass border-white/60 shadow-lg">
          <div className="border-b border-white/40 bg-white/40 p-4 flex items-center gap-2 text-sm font-bold text-slate-800"><Building2 className="h-4 w-4 text-primary" /> Institution Profile</div>
          <CardContent className="p-5 space-y-4 text-sm">
            <Field label="Institution Name"><input value={institution} onChange={(e) => setInstitution(e.target.value)} className="h-10 w-full rounded-lg border border-white/60 bg-white/60 px-3 text-sm font-semibold text-slate-700 focus:outline-none focus:ring-2 focus:ring-primary/40" /></Field>
            <Field label="University Affiliation"><input defaultValue="Savitribai Phule Pune University" className="h-10 w-full rounded-lg border border-white/60 bg-white/60 px-3 text-sm font-semibold text-slate-700 focus:outline-none focus:ring-2 focus:ring-primary/40" /></Field>
            <Field label="AISHE Institution Code"><input defaultValue="C-44187" className="h-10 w-full rounded-lg border border-white/60 bg-white/60 px-3 font-mono text-sm text-slate-700 focus:outline-none" /></Field>
          </CardContent>
        </Card>

        <Card className="glass border-white/60 shadow-lg">
          <div className="border-b border-white/40 bg-white/40 p-4 flex items-center gap-2 text-sm font-bold text-slate-800"><Landmark className="h-4 w-4 text-primary" /> Statutory Identifiers</div>
          <CardContent className="p-5 space-y-4 text-sm">
            <Field label="GSTIN"><input value={gstin} onChange={(e) => setGstin(e.target.value)} className="h-10 w-full rounded-lg border border-white/60 bg-white/60 px-3 font-mono text-sm text-slate-700 focus:outline-none" /></Field>
            <Field label="PAN"><input value={pan} onChange={(e) => setPan(e.target.value)} className="h-10 w-full rounded-lg border border-white/60 bg-white/60 px-3 font-mono text-sm text-slate-700 focus:outline-none" /></Field>
            <Field label="TAN"><input defaultValue="NAHS01234A" className="h-10 w-full rounded-lg border border-white/60 bg-white/60 px-3 font-mono text-sm text-slate-700 focus:outline-none" /></Field>
          </CardContent>
        </Card>

        <Card className="glass border-white/60 shadow-lg">
          <div className="border-b border-white/40 bg-white/40 p-4 flex items-center gap-2 text-sm font-bold text-slate-800"><CalendarDays className="h-4 w-4 text-primary" /> Fiscal Year</div>
          <CardContent className="p-5 space-y-4 text-sm">
            <Field label="Active Financial Year">
              <select value={fy} onChange={(e) => setFy(e.target.value)} className="h-10 w-full rounded-lg border border-white/60 bg-white/60 px-3 text-sm font-semibold text-slate-700 focus:outline-none">
                <option>2025-26</option><option>2026-27</option><option>2027-28</option>
              </select>
            </Field>
            <Field label="Period Closing Policy"><span className="text-xs text-slate-500">Books lock 15 days after period end · reopening requires CFO approval.</span></Field>
          </CardContent>
        </Card>

        <Card className="glass border-white/60 shadow-lg">
          <div className="border-b border-white/40 bg-white/40 p-4 flex items-center gap-2 text-sm font-bold text-slate-800"><Percent className="h-4 w-4 text-primary" /> Tax Defaults</div>
          <CardContent className="p-5 space-y-4 text-sm">
            <Field label="Default GST Rate"><select className="h-10 w-full rounded-lg border border-white/60 bg-white/60 px-3 text-sm font-semibold text-slate-700 focus:outline-none"><option>Exempt (education services)</option><option>5%</option><option>12%</option><option>18%</option></select></Field>
            <Field label="TDS Deduction Timing"><select className="h-10 w-full rounded-lg border border-white/60 bg-white/60 px-3 text-sm font-semibold text-slate-700 focus:outline-none"><option>At payment (accrual)</option><option>At invoice booking</option></select></Field>
            <p className="flex items-center gap-2 text-[11px] text-slate-400"><Settings2 className="h-3.5 w-3.5" /> Audit trail enabled — every change records actor, timestamp and prior value.</p>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return <div><label className="mb-1 block text-[10px] font-bold uppercase tracking-wider text-slate-500">{label}</label>{children}</div>;
}

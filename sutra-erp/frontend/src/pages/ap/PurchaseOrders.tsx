import { useState, useEffect, useMemo } from "react";
import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button } from "../../components/ui";
import { Plus, Trash2, Save } from "lucide-react";
import { api } from "../../lib/api-client";

interface POLine {
  id: string;
  itemDescription: string;
  quantity: string;
  unitPrice: string;
  taxRate: string;
}

interface Vendor {
  vendorId: string;
  vendorName: string;
}

export default function PurchaseOrders() {
  const [vendors, setVendors] = useState<Vendor[]>([]);
  const [selectedVendor, setSelectedVendor] = useState("");
  const [lines, setLines] = useState<POLine[]>([
    { id: "1", itemDescription: "", quantity: "", unitPrice: "", taxRate: "18" },
  ]);

  useEffect(() => {
    api.get<{data: Vendor[]}>('/ap/vendors').then(res => {
      if (res && res.data && res.data.length > 0) {
        setVendors(res.data);
      } else {
        setVendors([{ vendorId: "00000000-0000-0000-0000-000000000000", vendorName: "Select Vendor..." }]);
      }
    });
  }, []);

  const addLine = () => {
    setLines([...lines, { id: Date.now().toString(), itemDescription: "", quantity: "", unitPrice: "", taxRate: "18" }]);
  };

  const removeLine = (id: string) => {
    if (lines.length > 1) {
      setLines(lines.filter(l => l.id !== id));
    }
  };

  const updateLine = (id: string, field: keyof POLine, value: string) => {
    setLines(lines.map(l => (l.id === id ? { ...l, [field]: value } : l)));
  };

  const totals = useMemo(() => {
    let subtotal = 0;
    let tax = 0;
    lines.forEach(l => {
      const q = parseFloat(l.quantity) || 0;
      const p = parseFloat(l.unitPrice) || 0;
      const t = parseFloat(l.taxRate) || 0;
      const lineTotal = q * p;
      subtotal += lineTotal;
      tax += lineTotal * (t / 100);
    });
    return { subtotal, tax, net: subtotal + tax };
  }, [lines]);

  const handleSave = async () => {
    const payload = {
      entity_id: "00000000-0000-0000-0000-000000000000",
      vendor_id: selectedVendor || "00000000-0000-0000-0000-000000000000",
      order_date: new Date().toISOString().split('T')[0],
      delivery_date: new Date(Date.now() + 7 * 86400000).toISOString().split('T')[0],
      payment_terms: 30,
      lines: lines.map((l, i) => ({
        line_number: i + 1,
        item_description: l.itemDescription,
        hsn_sac_code: "9983",
        quantity: parseFloat(l.quantity) || 1,
        unit_price: parseFloat(l.unitPrice) || 0,
        tax_rate: parseFloat(l.taxRate) || 18,
        account_id: "00000000-0000-0000-0000-000000000000"
      }))
    };
    try {
      await api.post('/ap/purchase-orders', payload);
      alert("PO Created Successfully!");
      setLines([{ id: Date.now().toString(), itemDescription: "", quantity: "", unitPrice: "", taxRate: "18" }]);
    } catch (e) {
      alert("Error creating PO (check console)");
      console.error(e);
    }
  };

  return (
    <div className="animate-in fade-in duration-500 flex flex-col h-full">
      <PageHeader
        title="Create Purchase Order"
        description="Raise a new PO for an onboarded vendor"
        breadcrumbs={[{ label: "Accounts Payable" }, { label: "Purchase Orders" }]}
        actions={
          <Button size="sm" onClick={handleSave} className="bg-primary hover:bg-primary/90 text-white shadow-lg hover-lift">
            <Save className="h-4 w-4 mr-2" /> Save PO
          </Button>
        }
      />

      <Card className="mb-6 flex-1 flex flex-col">
        <div className="p-5 border-b border-white/60 bg-white/40 backdrop-blur-md grid grid-cols-3 gap-6">
          <div>
            <label className="block text-[10px] font-bold uppercase tracking-wider text-slate-500 mb-2">Vendor</label>
            <select 
              value={selectedVendor}
              onChange={(e) => setSelectedVendor(e.target.value)}
              className="w-full text-sm font-semibold glass-input rounded-md px-3 py-2"
            >
              <option value="">Select Vendor...</option>
              {vendors.map(v => <option key={v.vendorId} value={v.vendorId}>{v.vendorName}</option>)}
            </select>
          </div>
          <div>
            <label className="block text-[10px] font-bold uppercase tracking-wider text-slate-500 mb-2">Order Date</label>
            <input 
              type="date" 
              defaultValue={new Date().toISOString().split('T')[0]}
              className="w-full text-sm font-semibold glass-input rounded-md px-3 py-2"
            />
          </div>
          <div>
            <label className="block text-[10px] font-bold uppercase tracking-wider text-slate-500 mb-2">Delivery Date</label>
            <input 
              type="date" 
              defaultValue={new Date(Date.now() + 7 * 86400000).toISOString().split('T')[0]}
              className="w-full text-sm font-semibold glass-input rounded-md px-3 py-2"
            />
          </div>
        </div>
        <CardContent className="p-0 flex-1 overflow-auto custom-scrollbar relative">
          <table className="w-full text-left border-collapse">
            <thead className="sticky top-0 bg-white/80 backdrop-blur-md z-10 border-b border-white/60">
              <tr className="text-[10px] font-bold uppercase tracking-wider text-slate-500">
                <th className="p-3 w-10 text-center">#</th>
                <th className="p-3 border-l border-white/40">Item Description</th>
                <th className="p-3 w-32 border-l border-white/40 text-right">Qty</th>
                <th className="p-3 w-40 border-l border-white/40 text-right">Unit Price (₹)</th>
                <th className="p-3 w-32 border-l border-white/40 text-right">Tax %</th>
                <th className="p-3 w-40 border-l border-white/40 text-right">Total (₹)</th>
                <th className="p-3 w-12 border-l border-white/40 text-center"></th>
              </tr>
            </thead>
            <tbody>
              {lines.map((l, i) => (
                <tr key={l.id} className="border-b border-white/30 row-focus transition-all">
                  <td className="p-3 text-center text-xs font-bold text-slate-400">{i + 1}</td>
                  <td className="p-0 border-l border-white/30">
                    <input 
                      type="text" 
                      placeholder="Item details..."
                      value={l.itemDescription}
                      onChange={(e) => updateLine(l.id, "itemDescription", e.target.value)}
                      className="w-full h-full min-h-[44px] px-3 bg-transparent text-sm font-semibold focus:outline-none focus:bg-white/50 text-slate-800"
                    />
                  </td>
                  <td className="p-0 border-l border-white/30">
                    <input 
                      type="number" 
                      placeholder="0"
                      value={l.quantity}
                      onChange={(e) => updateLine(l.id, "quantity", e.target.value)}
                      className="w-full h-full min-h-[44px] px-3 text-right font-mono text-sm font-bold bg-transparent focus:outline-none focus:bg-white/50 text-slate-800"
                    />
                  </td>
                  <td className="p-0 border-l border-white/30">
                    <input 
                      type="number" 
                      placeholder="0.00"
                      value={l.unitPrice}
                      onChange={(e) => updateLine(l.id, "unitPrice", e.target.value)}
                      className="w-full h-full min-h-[44px] px-3 text-right font-mono text-sm font-bold bg-transparent focus:outline-none focus:bg-white/50 text-slate-800"
                    />
                  </td>
                  <td className="p-0 border-l border-white/30">
                    <select 
                      value={l.taxRate}
                      onChange={(e) => updateLine(l.id, "taxRate", e.target.value)}
                      className="w-full h-full min-h-[44px] px-3 text-right font-mono text-sm font-bold bg-transparent focus:outline-none focus:bg-white/50 text-slate-800"
                    >
                      <option value="0">0%</option>
                      <option value="5">5%</option>
                      <option value="12">12%</option>
                      <option value="18">18%</option>
                      <option value="28">28%</option>
                    </select>
                  </td>
                  <td className="p-3 text-right font-mono text-sm font-extrabold text-slate-800 bg-black/5 border-l border-white/30">
                    {((parseFloat(l.quantity) || 0) * (parseFloat(l.unitPrice) || 0) * (1 + (parseFloat(l.taxRate)||0)/100)).toLocaleString('en-IN', {minimumFractionDigits: 2})}
                  </td>
                  <td className="p-0 border-l border-white/30 align-middle">
                    <button 
                      onClick={() => removeLine(l.id)}
                      className="w-full h-full min-h-[44px] flex items-center justify-center text-slate-400 hover:text-destructive hover:bg-destructive/10 transition-colors"
                    >
                      <Trash2 className="h-4 w-4" />
                    </button>
                  </td>
                </tr>
              ))}
              <tr>
                <td colSpan={7} className="p-0">
                  <button 
                    onClick={addLine}
                    className="w-full p-3 flex items-center justify-center gap-2 text-xs font-bold text-primary hover:bg-primary/5 transition-colors"
                  >
                    <Plus className="h-4 w-4" /> Add Item Line
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
        </CardContent>
        <div className="bg-white/60 backdrop-blur-md border-t border-white/60 p-5 flex justify-end gap-12">
          <div className="text-right">
            <p className="text-[10px] font-bold uppercase tracking-widest text-slate-500 mb-1">Subtotal</p>
            <p className="font-mono text-lg font-bold text-slate-700">₹ {totals.subtotal.toLocaleString('en-IN', {minimumFractionDigits: 2})}</p>
          </div>
          <div className="text-right">
            <p className="text-[10px] font-bold uppercase tracking-widest text-slate-500 mb-1">Tax Amount</p>
            <p className="font-mono text-lg font-bold text-slate-700">₹ {totals.tax.toLocaleString('en-IN', {minimumFractionDigits: 2})}</p>
          </div>
          <div className="text-right">
            <p className="text-[10px] font-bold uppercase tracking-widest text-primary mb-1">Net Total</p>
            <p className="font-mono text-2xl font-extrabold text-slate-900">₹ {totals.net.toLocaleString('en-IN', {minimumFractionDigits: 2})}</p>
          </div>
        </div>
      </Card>
    </div>
  );
}

import { useState, useMemo, useRef, useEffect } from "react";
import { PageHeader } from "../../components/layout/PageHeader";
import { Card, CardContent, Button, Input, Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter, Spinner } from "../../components/ui";
import { Plus, Search, Edit2 } from "lucide-react";
import { useVendors, useCreateVendor, useUpdateVendor, type Vendor } from "../../api/ap/hooks";
import { useKeyboardShortcut, useShortcutDisplay } from "../../hooks/useKeyboardShortcut";

export default function Vendors() {
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const [editingVendor, setEditingVendor] = useState<Vendor | null>(null);
  const [search, setSearch] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  
  const { data: response, isLoading } = useVendors();
  const vendors = response?.data || [];

  const { format } = useShortcutDisplay();

  // Global shortcut to open create dialog
  useKeyboardShortcut('n', () => {
    setEditingVendor(null);
    setIsDialogOpen(true);
  }, { primary: true });

  const filteredVendors = useMemo(() => {
    if (!search.trim()) return vendors;
    const lower = search.toLowerCase();
    return vendors.filter((v: Vendor) => 
      v.vendorName?.toLowerCase().includes(lower) || 
      v.vendorCode?.toLowerCase().includes(lower) ||
      v.pan?.toLowerCase().includes(lower)
    );
  }, [vendors, search]);

  // Handle table row navigation
  useEffect(() => {
    const handleTableNav = (e: KeyboardEvent) => {
      if (isDialogOpen || filteredVendors.length === 0) return;
      
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setSelectedIndex((prev) => Math.min(prev + 1, filteredVendors.length - 1));
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        setSelectedIndex((prev) => Math.max(prev - 1, 0));
      } else if (e.key === 'Enter') {
        // Only if we aren't typing in the search box
        if (document.activeElement?.tagName !== 'INPUT') {
          e.preventDefault();
          handleEdit(filteredVendors[selectedIndex]);
        }
      }
    };

    window.addEventListener('keydown', handleTableNav);
    return () => window.removeEventListener('keydown', handleTableNav);
  }, [isDialogOpen, filteredVendors, selectedIndex]);

  // Reset selection when search changes
  useEffect(() => {
    setSelectedIndex(0);
  }, [search]);

  const handleEdit = (vendor: Vendor) => {
    setEditingVendor(vendor);
    setIsDialogOpen(true);
  };

  return (
    <div className="space-y-6">
      <PageHeader
        title="Vendor Master"
        description="Manage vendor onboarding, verification, and lifecycle"
        breadcrumbs={[{ label: "Accounts Payable", href: "/ap/vendors" }]}
        actions={
          <Button size="sm" onClick={() => { setEditingVendor(null); setIsDialogOpen(true); }}>
            <Plus className="h-4 w-4 mr-1" /> Onboard Vendor
            <span className="ml-2 text-[10px] opacity-70 bg-black/10 px-1 rounded">{format("Primary+N")}</span>
          </Button>
        }
      />
      
      <Card>
        <div className="p-4 border-b flex items-center justify-between bg-muted/20">
          <div className="relative w-72">
            <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
            <Input 
              placeholder="Search by Code, Name, PAN..." 
              className="pl-9 h-9 glass-input shadow-sm" 
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              onKeyDown={(e) => {
                // Allow enter to edit when focused on search box
                if (e.key === 'Enter' && filteredVendors.length > 0) {
                  e.preventDefault();
                  handleEdit(filteredVendors[selectedIndex]);
                }
              }}
            />
          </div>
          <div className="text-xs text-muted-foreground font-medium flex items-center gap-2">
            <kbd className="px-1.5 py-0.5 rounded border bg-muted">↑</kbd>
            <kbd className="px-1.5 py-0.5 rounded border bg-muted">↓</kbd> to navigate,
            <kbd className="px-1.5 py-0.5 rounded border bg-muted ml-1">Enter</kbd> to edit
          </div>
        </div>
        <CardContent className="p-0">
          {isLoading ? (
            <div className="flex justify-center p-12"><Spinner /></div>
          ) : vendors.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-12 text-sm text-muted-foreground">
              <p className="font-medium mb-1">No vendors found</p>
              <p>Click 'Onboard Vendor' to add your first vendor.</p>
            </div>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b bg-muted/50 text-left">
                    <th className="h-10 px-4 font-medium">Code</th>
                    <th className="h-10 px-4 font-medium">Name</th>
                    <th className="h-10 px-4 font-medium">Type</th>
                    <th className="h-10 px-4 font-medium">PAN</th>
                    <th className="h-10 px-4 font-medium">Status</th>
                    <th className="h-10 px-4 font-medium text-right">Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {filteredVendors.map((vendor: Vendor, idx: number) => (
                    <tr 
                      key={vendor.vendorId} 
                      className={`row-focus border-b cursor-pointer ${selectedIndex === idx ? 'active' : 'hover:bg-black/5 dark:hover:bg-white/5'}`}
                      onClick={() => { setSelectedIndex(idx); handleEdit(vendor); }}
                    >
                      <td className="p-4 font-medium">{vendor.vendorCode}</td>
                      <td className="p-4">{vendor.vendorName}</td>
                      <td className="p-4 text-muted-foreground">{vendor.vendorType}</td>
                      <td className="p-4">{vendor.pan || "-"}</td>
                      <td className="p-4">
                        <span className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${vendor.isActive ? 'bg-emerald-100 text-emerald-700' : 'bg-rose-100 text-rose-700'}`}>
                          {vendor.isActive ? "Active" : "Inactive"}
                        </span>
                      </td>
                      <td className="p-4 text-right">
                        <Button variant="ghost" size="sm" onClick={(e) => { e.stopPropagation(); handleEdit(vendor); }}>
                          <Edit2 className="h-4 w-4 text-muted-foreground hover:text-indigo-600" />
                        </Button>
                      </td>
                    </tr>
                  ))}
                  {filteredVendors.length === 0 && (
                    <tr>
                      <td colSpan={6} className="p-8 text-center text-muted-foreground">
                        No vendors match your search.
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          )}
        </CardContent>
      </Card>

      <VendorDialog 
        open={isDialogOpen} 
        onOpenChange={setIsDialogOpen} 
        vendor={editingVendor} 
      />
    </div>
  );
}

function VendorDialog({ open, onOpenChange, vendor }: { open: boolean, onOpenChange: (open: boolean) => void, vendor: Vendor | null }) {
  const createVendor = useCreateVendor();
  const updateVendor = useUpdateVendor();
  const formRef = useRef<HTMLFormElement>(null);

  const [formData, setFormData] = useState({
    vendorCode: "",
    vendorName: "",
    vendorType: "COMPANY",
    pan: "",
    gstin: "",
    paymentTerms: 30,
  });

  // Populate data when editing
  useEffect(() => {
    if (open) {
      if (vendor) {
        setFormData({
          vendorCode: vendor.vendorCode || "",
          vendorName: vendor.vendorName || "",
          vendorType: vendor.vendorType || "COMPANY",
          pan: vendor.pan || "",
          gstin: vendor.gstin || "",
          paymentTerms: vendor.paymentTerms || 30,
        });
      } else {
        setFormData({ vendorCode: "", vendorName: "", vendorType: "COMPANY", pan: "", gstin: "", paymentTerms: 30 });
      }
    }
  }, [open, vendor]);

  // Focus management: Focus first input on open
  useEffect(() => {
    if (open) {
      setTimeout(() => {
        const firstInput = formRef.current?.querySelector('input, select') as HTMLElement;
        if (firstInput) firstInput.focus();
      }, 50);
    }
  }, [open]);

  // Tally-style Enter navigation
  const handleKeyDown = (e: React.KeyboardEvent<HTMLFormElement>) => {
    if (e.key === 'Enter') {
      const form = formRef.current;
      if (!form) return;
      
      const elements = Array.from(form.elements).filter(
        el => (el as HTMLInputElement).type !== 'hidden' && !el.hasAttribute('disabled')
      ) as HTMLElement[];
      
      const index = elements.indexOf(e.target as HTMLElement);
      
      // If pressing enter on a button, let default behavior happen (e.g. submit)
      if ((e.target as HTMLElement).tagName === 'BUTTON') {
        return; 
      }

      e.preventDefault();

      if (index > -1 && index < elements.length - 1) {
        // Find next input/select/button
        elements[index + 1].focus();
      } else if (index === elements.length - 1) {
        // Last element (submit button), submit form
        form.requestSubmit();
      }
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      if (vendor) {
        await updateVendor.mutateAsync({ id: vendor.vendorId, data: formData });
      } else {
        await createVendor.mutateAsync({ ...formData, entity_id: null });
      }
      onOpenChange(false);
    } catch (err) {
      console.error("Failed to save vendor", err);
      alert("Failed to save vendor");
    }
  };

  const isPending = createVendor.isPending || updateVendor.isPending;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>{vendor ? "Edit Vendor" : "Onboard Vendor"}</DialogTitle>
        </DialogHeader>
        <form ref={formRef} onSubmit={handleSubmit} onKeyDown={handleKeyDown} className="space-y-4 py-4">
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">Vendor Code</label>
              <Input 
                required 
                value={formData.vendorCode} 
                onChange={(e) => setFormData({...formData, vendorCode: e.target.value})}
                placeholder="e.g. VEN-001" 
              />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">Vendor Type</label>
              <select 
                className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50"
                value={formData.vendorType}
                onChange={(e) => setFormData({...formData, vendorType: e.target.value})}
              >
                <option value="COMPANY">Company</option>
                <option value="INDIVIDUAL">Individual</option>
                <option value="PARTNERSHIP">Partnership</option>
                <option value="HUF">HUF</option>
              </select>
            </div>
          </div>
          <div className="space-y-2">
            <label className="text-sm font-medium">Vendor Name</label>
            <Input 
              required 
              value={formData.vendorName} 
              onChange={(e) => setFormData({...formData, vendorName: e.target.value})}
              placeholder="Full registered name" 
            />
          </div>
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">PAN</label>
              <Input 
                value={formData.pan} 
                onChange={(e) => setFormData({...formData, pan: e.target.value})}
                placeholder="10-digit PAN" 
                maxLength={10}
              />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">GSTIN</label>
              <Input 
                value={formData.gstin} 
                onChange={(e) => setFormData({...formData, gstin: e.target.value})}
                placeholder="15-digit GSTIN" 
                maxLength={15}
              />
            </div>
          </div>
          <div className="space-y-2">
            <label className="text-sm font-medium">Payment Terms (Days)</label>
            <Input 
              type="number"
              value={formData.paymentTerms} 
              onChange={(e) => setFormData({...formData, paymentTerms: parseInt(e.target.value) || 0})}
            />
          </div>
          <DialogFooter className="pt-4 flex items-center justify-end">
            <div className="mr-auto text-xs text-muted-foreground flex items-center gap-1">
              <kbd className="bg-muted px-1.5 py-0.5 rounded border shadow-sm">↵ Enter</kbd> to navigate
            </div>
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button type="submit" disabled={isPending}>
              {isPending ? "Saving..." : "Save Vendor"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

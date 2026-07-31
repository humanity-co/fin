import { create } from "zustand";

interface UIState {
  /** Sidebar collapsed state */
  sidebarCollapsed: boolean;
  toggleSidebar: () => void;
  setSidebarCollapsed: (collapsed: boolean) => void;

  /** Theme */
  theme: "light" | "dark" | "system";
  setTheme: (theme: "light" | "dark" | "system") => void;

  /** Active entity (campus) */
  activeEntityId: string | null;
  setActiveEntityId: (id: string | null) => void;
}

export const useUIStore = create<UIState>((set) => ({
  sidebarCollapsed: false,
  toggleSidebar: () =>
    set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),
  setSidebarCollapsed: (collapsed) => set({ sidebarCollapsed: collapsed }),

  theme: (localStorage.getItem("sutra-theme") as UIState["theme"]) || "light",
  setTheme: (theme) => {
    localStorage.setItem("sutra-theme", theme);
    set({ theme });
  },

  activeEntityId: null,
  setActiveEntityId: (id) => set({ activeEntityId: id }),
}));

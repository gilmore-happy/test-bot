import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface UIState {
  theme: 'dark' | 'light';
  sidebarCollapsed: boolean;
  refreshRate: number; // in seconds
  toggleTheme: () => void;
  toggleSidebar: () => void;
  setRefreshRate: (rate: number) => void;
}

export const useUIStore = create<UIState>()(
  persist(
    (set) => ({
      theme: 'dark',
      sidebarCollapsed: false,
      refreshRate: 5, // 5 seconds default
      
      toggleTheme: () => {
        set(state => {
          const newTheme = state.theme === 'dark' ? 'light' : 'dark';
          // Apply theme to document
          document.documentElement.classList.toggle('dark', newTheme === 'dark');
          return { theme: newTheme };
        });
      },
      
      toggleSidebar: () => {
        set(state => ({ sidebarCollapsed: !state.sidebarCollapsed }));
      },
      
      setRefreshRate: (rate) => {
        set({ refreshRate: rate });
      },
    }),
    {
      name: 'solana-hft-ui-settings',
    }
  )
);

// Initialize theme on app load
if (typeof window !== 'undefined') {
  const storedState = JSON.parse(
    localStorage.getItem('solana-hft-ui-settings') || '{"state":{"theme":"dark"}}'
  );
  document.documentElement.classList.toggle('dark', storedState.state.theme === 'dark');
}
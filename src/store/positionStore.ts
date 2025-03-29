import { create } from 'zustand';
import { Position } from '../types';
import { getPositions, getPosition, closePosition, getPositionHistory } from '../api/positions';

interface PositionState {
  positions: Position[];
  currentPosition: Position | null;
  positionHistory: Position[];
  loading: boolean;
  error: string | null;
  filters: {
    status?: 'open' | 'closed' | 'liquidated';
    asset?: string;
    strategy?: string;
    startDate?: string;
    endDate?: string;
  };
  fetchPositions: () => Promise<void>;
  fetchPosition: (id: string) => Promise<void>;
  closePositionById: (id: string) => Promise<void>;
  fetchPositionHistory: () => Promise<void>;
  setFilters: (filters: PositionState['filters']) => void;
}

export const usePositionStore = create<PositionState>((set, get) => ({
  positions: [],
  currentPosition: null,
  positionHistory: [],
  loading: false,
  error: null,
  filters: {
    status: 'open',
  },
  
  fetchPositions: async () => {
    set({ loading: true, error: null });
    try {
      const response = await getPositions(get().filters);
      if (response.success && response.data) {
        set({ positions: response.data, loading: false });
      } else {
        set({ error: response.error || 'Failed to fetch positions', loading: false });
      }
    } catch (error: any) {
      set({ error: error.message || 'An error occurred', loading: false });
    }
  },
  
  fetchPosition: async (id: string) => {
    set({ loading: true, error: null });
    try {
      const response = await getPosition(id);
      if (response.success && response.data) {
        set({ currentPosition: response.data, loading: false });
      } else {
        set({ error: response.error || 'Failed to fetch position', loading: false });
      }
    } catch (error: any) {
      set({ error: error.message || 'An error occurred', loading: false });
    }
  },
  
  closePositionById: async (id: string) => {
    set({ loading: true, error: null });
    try {
      const response = await closePosition(id);
      if (response.success) {
        // Update the position in the list
        const positions = get().positions.map(position => 
          position.id === id ? { ...position, status: 'closed' as const } : position
        );
        
        // Update current position if it's the one being closed
        const currentPosition = get().currentPosition;
        if (currentPosition && currentPosition.id === id) {
          set({ 
            positions, 
            currentPosition: { ...currentPosition, status: 'closed' as const },
            loading: false 
          });
        } else {
          set({ positions, loading: false });
        }
      } else {
        set({ error: response.error || 'Failed to close position', loading: false });
      }
    } catch (error: any) {
      set({ error: error.message || 'An error occurred', loading: false });
    }
  },
  
  fetchPositionHistory: async () => {
    set({ loading: true, error: null });
    try {
      const { status, ...historyFilters } = get().filters;
      const response = await getPositionHistory(historyFilters);
      if (response.success && response.data) {
        set({ positionHistory: response.data, loading: false });
      } else {
        set({ error: response.error || 'Failed to fetch position history', loading: false });
      }
    } catch (error: any) {
      set({ error: error.message || 'An error occurred', loading: false });
    }
  },
  
  setFilters: (filters) => {
    set({ filters: { ...get().filters, ...filters } });
  },
}));
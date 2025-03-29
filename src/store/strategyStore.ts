import { create } from 'zustand';
import { Strategy } from '../types';
import { 
  getStrategies, 
  getStrategy, 
  updateStrategyStatus, 
  updateStrategyParameters,
  updateStrategySchedule
} from '../api/strategies';

interface StrategyState {
  strategies: Strategy[];
  currentStrategy: Strategy | null;
  loading: boolean;
  error: string | null;
  fetchStrategies: () => Promise<void>;
  fetchStrategy: (id: string) => Promise<void>;
  updateStatus: (id: string, status: 'active' | 'paused' | 'stopped') => Promise<void>;
  updateParameters: (id: string, parameters: Record<string, any>) => Promise<void>;
  updateSchedule: (
    id: string,
    schedule: {
      active: boolean;
      startTime: string;
      endTime: string;
      days: ('monday' | 'tuesday' | 'wednesday' | 'thursday' | 'friday' | 'saturday' | 'sunday')[];
    }
  ) => Promise<void>;
}

export const useStrategyStore = create<StrategyState>((set, get) => ({
  strategies: [],
  currentStrategy: null,
  loading: false,
  error: null,
  
  fetchStrategies: async () => {
    set({ loading: true, error: null });
    try {
      const response = await getStrategies();
      if (response.success && response.data) {
        set({ strategies: response.data, loading: false });
      } else {
        set({ error: response.error || 'Failed to fetch strategies', loading: false });
      }
    } catch (error: any) {
      set({ error: error.message || 'An error occurred', loading: false });
    }
  },
  
  fetchStrategy: async (id: string) => {
    set({ loading: true, error: null });
    try {
      const response = await getStrategy(id);
      if (response.success && response.data) {
        set({ currentStrategy: response.data, loading: false });
      } else {
        set({ error: response.error || 'Failed to fetch strategy', loading: false });
      }
    } catch (error: any) {
      set({ error: error.message || 'An error occurred', loading: false });
    }
  },
  
  updateStatus: async (id: string, status: 'active' | 'paused' | 'stopped') => {
    set({ loading: true, error: null });
    try {
      const response = await updateStrategyStatus(id, status);
      if (response.success && response.data) {
        // Update the strategy in the list
        const strategies = get().strategies.map((strategy: Strategy) =>
          strategy.id === id ? { ...strategy, status } : strategy
        );
        
        // Update current strategy if it's the one being modified
        const currentStrategy = get().currentStrategy;
        if (currentStrategy && currentStrategy.id === id) {
          set({ 
            strategies, 
            currentStrategy: { ...currentStrategy, status },
            loading: false 
          });
        } else {
          set({ strategies, loading: false });
        }
      } else {
        set({ error: response.error || 'Failed to update strategy status', loading: false });
      }
    } catch (error: any) {
      set({ error: error.message || 'An error occurred', loading: false });
    }
  },
  
  updateParameters: async (id: string, parameters: Record<string, any>) => {
    set({ loading: true, error: null });
    try {
      const response = await updateStrategyParameters(id, parameters);
      if (response.success && response.data) {
        // Update the strategy in the list
        const strategies = get().strategies.map((strategy: Strategy) =>
          strategy.id === id ? { ...strategy, parameters: { ...strategy.parameters, ...parameters } } : strategy
        );
        
        // Update current strategy if it's the one being modified
        const currentStrategy = get().currentStrategy;
        if (currentStrategy && currentStrategy.id === id) {
          set({ 
            strategies, 
            currentStrategy: { 
              ...currentStrategy, 
              parameters: { ...currentStrategy.parameters, ...parameters } 
            },
            loading: false 
          });
        } else {
          set({ strategies, loading: false });
        }
      } else {
        set({ error: response.error || 'Failed to update strategy parameters', loading: false });
      }
    } catch (error: any) {
      set({ error: error.message || 'An error occurred', loading: false });
    }
  },
  
  updateSchedule: async (
    id: string,
    schedule: {
      active: boolean;
      startTime: string;
      endTime: string;
      days: ('monday' | 'tuesday' | 'wednesday' | 'thursday' | 'friday' | 'saturday' | 'sunday')[];
    }
  ) => {
    set({ loading: true, error: null });
    try {
      const response = await updateStrategySchedule(id, schedule);
      if (response.success && response.data) {
        // Update the strategy in the list
        const strategies = get().strategies.map((strategy: Strategy) =>
          strategy.id === id ? { ...strategy, schedule } : strategy
        );
        
        // Update current strategy if it's the one being modified
        const currentStrategy = get().currentStrategy;
        if (currentStrategy && currentStrategy.id === id) {
          set({ 
            strategies, 
            currentStrategy: { ...currentStrategy, schedule },
            loading: false 
          });
        } else {
          set({ strategies, loading: false });
        }
      } else {
        set({ error: response.error || 'Failed to update strategy schedule', loading: false });
      }
    } catch (error: any) {
      set({ error: error.message || 'An error occurred', loading: false });
    }
  },
}));
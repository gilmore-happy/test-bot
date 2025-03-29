import { create } from 'zustand';
import { Asset, AssetHistory } from '../types';
import { getAssets, getAsset, getAssetHistory, depositAsset, withdrawAsset } from '../api/assets';

interface AssetState {
  assets: Asset[];
  currentAsset: Asset | null;
  assetHistory: AssetHistory[];
  loading: boolean;
  error: string | null;
  fetchAssets: () => Promise<void>;
  fetchAsset: (symbol: string) => Promise<void>;
  fetchAssetHistory: (timeframe?: 'day' | 'week' | 'month' | 'year') => Promise<void>;
  deposit: (symbol: string, amount: number, address: string) => Promise<boolean>;
  withdraw: (symbol: string, amount: number, address: string) => Promise<boolean>;
}

export const useAssetStore = create<AssetState>((set, get) => ({
  assets: [],
  currentAsset: null,
  assetHistory: [],
  loading: false,
  error: null,
  
  fetchAssets: async () => {
    set({ loading: true, error: null });
    try {
      const response = await getAssets();
      if (response.success && response.data) {
        set({ assets: response.data, loading: false });
      } else {
        set({ error: response.error || 'Failed to fetch assets', loading: false });
      }
    } catch (error: any) {
      set({ error: error.message || 'An error occurred', loading: false });
    }
  },
  
  fetchAsset: async (symbol: string) => {
    set({ loading: true, error: null });
    try {
      const response = await getAsset(symbol);
      if (response.success && response.data) {
        set({ currentAsset: response.data, loading: false });
      } else {
        set({ error: response.error || 'Failed to fetch asset', loading: false });
      }
    } catch (error: any) {
      set({ error: error.message || 'An error occurred', loading: false });
    }
  },
  
  fetchAssetHistory: async (timeframe: 'day' | 'week' | 'month' | 'year' = 'week') => {
    set({ loading: true, error: null });
    try {
      const response = await getAssetHistory(timeframe);
      if (response.success && response.data) {
        set({ assetHistory: response.data, loading: false });
      } else {
        set({ error: response.error || 'Failed to fetch asset history', loading: false });
      }
    } catch (error: any) {
      set({ error: error.message || 'An error occurred', loading: false });
    }
  },
  
  deposit: async (symbol: string, amount: number, address: string) => {
    set({ loading: true, error: null });
    try {
      const response = await depositAsset(symbol, amount, address);
      set({ loading: false });
      if (response.success) {
        // Refresh assets after successful deposit
        get().fetchAssets();
        return true;
      } else {
        set({ error: response.error || 'Failed to deposit asset' });
        return false;
      }
    } catch (error: any) {
      set({ error: error.message || 'An error occurred', loading: false });
      return false;
    }
  },
  
  withdraw: async (symbol: string, amount: number, address: string) => {
    set({ loading: true, error: null });
    try {
      const response = await withdrawAsset(symbol, amount, address);
      set({ loading: false });
      if (response.success) {
        // Refresh assets after successful withdrawal
        get().fetchAssets();
        return true;
      } else {
        set({ error: response.error || 'Failed to withdraw asset' });
        return false;
      }
    } catch (error: any) {
      set({ error: error.message || 'An error occurred', loading: false });
      return false;
    }
  },
}));
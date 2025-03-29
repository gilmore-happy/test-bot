import { create } from 'zustand';
import {
  SystemStatus,
  PerformanceMetrics,
  BotConfig,
  HardwareCapabilities,
  FeatureFlags,
  HardwareConfig
} from '../types';
import {
  getSystemStatus,
  getPerformanceMetrics,
  getSystemConfig,
  updateSystemConfig,
  getHardwareCapabilities,
  getHardwareProfiles,
  applyHardwareProfile,
  getFeatureFlags,
  enableFeature,
  disableFeature
} from '../api/system';

interface SystemState {
  status: SystemStatus | null;
  performanceMetrics: PerformanceMetrics | null;
  config: BotConfig | null;
  hardwareCapabilities: HardwareCapabilities | null;
  hardwareProfiles: string[];
  featureFlags: FeatureFlags | null;
  loading: boolean;
  error: string | null;
  fetchStatus: () => Promise<void>;
  fetchPerformanceMetrics: () => Promise<void>;
  fetchConfig: () => Promise<void>;
  updateConfig: (config: Partial<BotConfig>) => Promise<void>;
  fetchHardwareCapabilities: () => Promise<void>;
  fetchHardwareProfiles: () => Promise<void>;
  applyProfile: (profileName: string) => Promise<void>;
  fetchFeatureFlags: () => Promise<void>;
  toggleFeature: (featureName: string, enabled: boolean) => Promise<void>;
  updateHardwareConfig: (hardwareConfig: Partial<HardwareConfig>) => Promise<void>;
}

export const useSystemStore = create<SystemState>((set, get) => ({
  status: null,
  performanceMetrics: null,
  config: null,
  hardwareCapabilities: null,
  hardwareProfiles: [],
  featureFlags: null,
  loading: false,
  error: null,
  
  fetchStatus: async () => {
    set({ loading: true, error: null });
    try {
      const response = await getSystemStatus();
      if (response.success && response.data) {
        set({ status: response.data, loading: false });
      } else {
        set({ error: response.error || 'Failed to fetch system status', loading: false });
      }
    } catch (error: any) {
      set({ error: error.message || 'An error occurred', loading: false });
    }
  },
  
  fetchPerformanceMetrics: async () => {
    set({ loading: true, error: null });
    try {
      const response = await getPerformanceMetrics();
      if (response.success && response.data) {
        set({ performanceMetrics: response.data, loading: false });
      } else {
        set({ error: response.error || 'Failed to fetch performance metrics', loading: false });
      }
    } catch (error: any) {
      set({ error: error.message || 'An error occurred', loading: false });
    }
  },

  fetchConfig: async () => {
    set({ loading: true, error: null });
    try {
      const config = await getSystemConfig();
      set({ config, loading: false });
    } catch (error: any) {
      set({ error: error.message || 'Failed to fetch system configuration', loading: false });
    }
  },

  updateConfig: async (configUpdate) => {
    set({ loading: true, error: null });
    try {
      const updatedConfig = await updateSystemConfig(configUpdate);
      set({ config: updatedConfig, loading: false });
    } catch (error: any) {
      set({ error: error.message || 'Failed to update system configuration', loading: false });
    }
  },

  fetchHardwareCapabilities: async () => {
    set({ loading: true, error: null });
    try {
      const capabilities = await getHardwareCapabilities();
      set({ hardwareCapabilities: capabilities, loading: false });
    } catch (error: any) {
      set({ error: error.message || 'Failed to fetch hardware capabilities', loading: false });
    }
  },

  fetchHardwareProfiles: async () => {
    set({ loading: true, error: null });
    try {
      const profiles = await getHardwareProfiles();
      set({ hardwareProfiles: profiles, loading: false });
    } catch (error: any) {
      set({ error: error.message || 'Failed to fetch hardware profiles', loading: false });
    }
  },

  applyProfile: async (profileName) => {
    set({ loading: true, error: null });
    try {
      const updatedConfig = await applyHardwareProfile(profileName);
      set({ config: updatedConfig, loading: false });
    } catch (error: any) {
      set({ error: error.message || 'Failed to apply hardware profile', loading: false });
    }
  },

  fetchFeatureFlags: async () => {
    set({ loading: true, error: null });
    try {
      const flags = await getFeatureFlags();
      set({ featureFlags: flags, loading: false });
    } catch (error: any) {
      set({ error: error.message || 'Failed to fetch feature flags', loading: false });
    }
  },

  toggleFeature: async (featureName, enabled) => {
    set({ loading: true, error: null });
    try {
      const updatedFlags = enabled
        ? await enableFeature(featureName)
        : await disableFeature(featureName);
      set({ featureFlags: updatedFlags, loading: false });
    } catch (error: any) {
      set({ error: error.message || 'Failed to toggle feature', loading: false });
    }
  },

  updateHardwareConfig: async (hardwareConfig) => {
    const { config } = get();
    if (!config) {
      set({ error: 'Cannot update hardware config: System config not loaded', loading: false });
      return;
    }

    set({ loading: true, error: null });
    try {
      const updatedConfig = await updateSystemConfig({
        hardware: { ...config.hardware, ...hardwareConfig }
      });
      set({ config: updatedConfig, loading: false });
    } catch (error: any) {
      set({ error: error.message || 'Failed to update hardware configuration', loading: false });
    }
  },
}));
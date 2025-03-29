import { apiRequest } from './client';
import {
  BotConfig,
  HardwareCapabilities,
  FeatureFlags,
  PerformanceMetrics,
  SystemStatus
} from '../types';

/**
 * Get system status
 */
export const getSystemStatus = async () => {
  const response = await apiRequest<SystemStatus>({ method: 'GET', url: '/api/system/status' });
  return response;
};

/**
 * Get performance metrics
 */
export const getPerformanceMetrics = async () => {
  const response = await apiRequest<PerformanceMetrics>({ method: 'GET', url: '/api/system/performance' });
  return response;
};

/**
 * Get system configuration
 */
export const getSystemConfig = async (): Promise<BotConfig> => {
  const response = await apiRequest<BotConfig>({ method: 'GET', url: '/api/system/config' });
  if (!response.success || !response.data) {
    throw new Error(response.error || 'Failed to get system configuration');
  }
  return response.data;
};

/**
 * Update system configuration
 */
export const updateSystemConfig = async (config: Partial<BotConfig>): Promise<BotConfig> => {
  const response = await apiRequest<BotConfig>({ method: 'POST', url: '/api/system/config', data: config });
  if (!response.success || !response.data) {
    throw new Error(response.error || 'Failed to update system configuration');
  }
  return response.data;
};

/**
 * Get hardware capabilities
 */
export const getHardwareCapabilities = async (): Promise<HardwareCapabilities> => {
  const response = await apiRequest<HardwareCapabilities>({ method: 'GET', url: '/api/system/hardware' });
  if (!response.success || !response.data) {
    throw new Error(response.error || 'Failed to get hardware capabilities');
  }
  return response.data;
};

/**
 * Get available hardware profiles
 */
export const getHardwareProfiles = async (): Promise<string[]> => {
  const response = await apiRequest<string[]>({ method: 'GET', url: '/api/system/profiles' });
  if (!response.success || !response.data) {
    throw new Error(response.error || 'Failed to get hardware profiles');
  }
  return response.data;
};

/**
 * Apply hardware profile
 */
export const applyHardwareProfile = async (profileName: string): Promise<BotConfig> => {
  const response = await apiRequest<BotConfig>({
    method: 'POST',
    url: '/api/system/profiles/apply',
    data: { profileName }
  });
  if (!response.success || !response.data) {
    throw new Error(response.error || 'Failed to apply hardware profile');
  }
  return response.data;
};

/**
 * Get feature flags
 */
export const getFeatureFlags = async (): Promise<FeatureFlags> => {
  const response = await apiRequest<FeatureFlags>({ method: 'GET', url: '/api/system/features' });
  if (!response.success || !response.data) {
    throw new Error(response.error || 'Failed to get feature flags');
  }
  return response.data;
};

/**
 * Enable feature
 */
export const enableFeature = async (featureName: string): Promise<FeatureFlags> => {
  const response = await apiRequest<FeatureFlags>({
    method: 'POST',
    url: '/api/system/features/enable',
    data: { featureName }
  });
  if (!response.success || !response.data) {
    throw new Error(response.error || 'Failed to enable feature');
  }
  return response.data;
};

/**
 * Disable feature
 */
export const disableFeature = async (featureName: string): Promise<FeatureFlags> => {
  const response = await apiRequest<FeatureFlags>({
    method: 'POST',
    url: '/api/system/features/disable',
    data: { featureName }
  });
  if (!response.success || !response.data) {
    throw new Error(response.error || 'Failed to disable feature');
  }
  return response.data;
};

/**
 * Get network endpoints
 */
export const getNetworkEndpoints = async () => {
  const response = await apiRequest<any[]>({ method: 'GET', url: '/api/system/network/endpoints' });
  return response;
};

/**
 * Get endpoint statistics
 */
export const getEndpointStats = async () => {
  const response = await apiRequest<any>({ method: 'GET', url: '/api/system/network/endpoints/stats' });
  return response;
};

/**
 * Set primary endpoint
 */
export const setPrimaryEndpoint = async (url: string) => {
  const response = await apiRequest<any>({
    method: 'POST',
    url: '/api/system/network/endpoints/primary',
    data: { url }
  });
  return response;
};

/**
 * Reset primary endpoint (use automatic selection)
 */
export const resetPrimaryEndpoint = async () => {
  const response = await apiRequest<any>({ method: 'POST', url: '/api/system/network/endpoints/reset' });
  return response;
};

/**
 * Save configuration to file
 */
export const saveConfigToFile = async (path?: string) => {
  const response = await apiRequest<any>({
    method: 'POST',
    url: '/api/system/config/save',
    data: { path }
  });
  return response;
};

/**
 * Load configuration from file
 */
export const loadConfigFromFile = async (path: string) => {
  const response = await apiRequest<BotConfig>({
    method: 'POST',
    url: '/api/system/config/load',
    data: { path }
  });
  return response;
};

/**
 * Generate default configuration
 */
export const generateDefaultConfig = async () => {
  const response = await apiRequest<BotConfig>({ method: 'POST', url: '/api/system/config/default' });
  return response;
};

/**
 * Restart system with new configuration
 */
export const restartSystem = async () => {
  const response = await apiRequest<any>({ method: 'POST', url: '/api/system/restart' });
  return response;
};
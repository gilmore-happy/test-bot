import { apiRequest } from './client';
import { Asset, AssetHistory } from '../types';

export const getAssets = () => {
  return apiRequest<Asset[]>({
    method: 'GET',
    url: '/assets',
  });
};

export const getAsset = (symbol: string) => {
  return apiRequest<Asset>({
    method: 'GET',
    url: `/assets/${symbol}`,
  });
};

export const getAssetHistory = (
  timeframe: 'day' | 'week' | 'month' | 'year' = 'week'
) => {
  return apiRequest<AssetHistory[]>({
    method: 'GET',
    url: '/assets/history',
    params: { timeframe },
  });
};

export const depositAsset = (symbol: string, amount: number, address: string) => {
  return apiRequest<{ success: boolean; txHash?: string }>({
    method: 'POST',
    url: `/assets/${symbol}/deposit`,
    data: { amount, address },
  });
};

export const withdrawAsset = (symbol: string, amount: number, address: string) => {
  return apiRequest<{ success: boolean; txHash?: string }>({
    method: 'POST',
    url: `/assets/${symbol}/withdraw`,
    data: { amount, address },
  });
};
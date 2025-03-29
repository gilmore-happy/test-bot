import { apiRequest } from './client';
import { Position } from '../types';

export const getPositions = (filters?: {
  status?: 'open' | 'closed' | 'liquidated';
  asset?: string;
  strategy?: string;
  startDate?: string;
  endDate?: string;
}) => {
  return apiRequest<Position[]>({
    method: 'GET',
    url: '/positions',
    params: filters,
  });
};

export const getPosition = (id: string) => {
  return apiRequest<Position>({
    method: 'GET',
    url: `/positions/${id}`,
  });
};

export const closePosition = (id: string) => {
  return apiRequest<{ success: boolean }>({
    method: 'POST',
    url: `/positions/${id}/close`,
  });
};

export const getPositionHistory = (
  filters?: {
    asset?: string;
    strategy?: string;
    startDate?: string;
    endDate?: string;
  }
) => {
  return apiRequest<Position[]>({
    method: 'GET',
    url: '/positions/history',
    params: filters,
  });
};
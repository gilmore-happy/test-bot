import { apiRequest } from './client';
import { Strategy } from '../types';

export const getStrategies = () => {
  return apiRequest<Strategy[]>({
    method: 'GET',
    url: '/strategies',
  });
};

export const getStrategy = (id: string) => {
  return apiRequest<Strategy>({
    method: 'GET',
    url: `/strategies/${id}`,
  });
};

export const updateStrategyStatus = (id: string, status: 'active' | 'paused' | 'stopped') => {
  return apiRequest<Strategy>({
    method: 'PATCH',
    url: `/strategies/${id}/status`,
    data: { status },
  });
};

export const updateStrategyParameters = (id: string, parameters: Record<string, any>) => {
  return apiRequest<Strategy>({
    method: 'PATCH',
    url: `/strategies/${id}/parameters`,
    data: { parameters },
  });
};

export const updateStrategySchedule = (
  id: string,
  schedule: {
    active: boolean;
    startTime: string;
    endTime: string;
    days: string[];
  }
) => {
  return apiRequest<Strategy>({
    method: 'PATCH',
    url: `/strategies/${id}/schedule`,
    data: { schedule },
  });
};
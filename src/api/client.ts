import axios, { AxiosInstance, AxiosRequestConfig } from 'axios';
import { ApiResponse } from '../types';

// Create a base API client
const createApiClient = (): AxiosInstance => {
  const client = axios.create({
    baseURL: process.env.REACT_APP_API_URL || 'http://localhost:8080/api',
    timeout: 10000,
    headers: {
      'Content-Type': 'application/json',
    },
  });

  // Request interceptor
  client.interceptors.request.use(
    (config) => {
      // You can add auth token here if needed
      const token = localStorage.getItem('auth_token');
      if (token) {
        config.headers.Authorization = `Bearer ${token}`;
      }
      return config;
    },
    (error) => {
      return Promise.reject(error);
    }
  );

  // Response interceptor
  client.interceptors.response.use(
    (response) => {
      return response;
    },
    (error) => {
      // Handle global error responses here
      if (error.response?.status === 401) {
        // Handle unauthorized access
        localStorage.removeItem('auth_token');
        window.location.href = '/login';
      }
      return Promise.reject(error);
    }
  );

  return client;
};

// API client singleton
export const apiClient = createApiClient();

// Generic API request function
export const apiRequest = async <T>(
  config: AxiosRequestConfig
): Promise<ApiResponse<T>> => {
  try {
    const response = await apiClient(config);
    return response.data as ApiResponse<T>;
  } catch (error: any) {
    if (error.response) {
      // The request was made and the server responded with a status code
      // that falls out of the range of 2xx
      return {
        success: false,
        error: error.response.data.message || 'An error occurred',
        timestamp: new Date().toISOString(),
      };
    } else if (error.request) {
      // The request was made but no response was received
      return {
        success: false,
        error: 'No response from server',
        timestamp: new Date().toISOString(),
      };
    } else {
      // Something happened in setting up the request that triggered an Error
      return {
        success: false,
        error: error.message || 'An unknown error occurred',
        timestamp: new Date().toISOString(),
      };
    }
  }
};
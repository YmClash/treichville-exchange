import apiClient from './client';
import {
  Transaction,
  InitiateExchangeDto,
  ConfirmPaymentDto,
  TransactionStatistics,
} from '@/lib/types/transaction';
import { PaginatedResponse } from '@/lib/types/api';

export const exchangeApi = {
  // Initiate new exchange transaction
  initiateTransaction: async (data: InitiateExchangeDto): Promise<Transaction> => {
    const response = await apiClient.post('/api/v1/exchange/initiate', data);
    return response.data;
  },

  // Confirm payment for transaction
  confirmPayment: async (data: ConfirmPaymentDto): Promise<Transaction> => {
    const response = await apiClient.post('/api/v1/exchange/confirm', data);
    return response.data;
  },

  // Complete transaction
  completeTransaction: async (id: string): Promise<Transaction> => {
    const response = await apiClient.post(`/api/v1/exchange/${id}/complete`);
    return response.data;
  },

  // Cancel transaction
  cancelTransaction: async (id: string, reason?: string): Promise<Transaction> => {
    const response = await apiClient.post(`/api/v1/exchange/${id}/cancel`, {
      reason,
    });
    return response.data;
  },

  // Get transaction by ID
  getTransaction: async (id: string): Promise<Transaction> => {
    const response = await apiClient.get(`/api/v1/exchange/${id}`);
    return response.data;
  },

  // Get user's transactions
  getUserTransactions: async (params?: {
    status?: string;
    from_date?: string;
    to_date?: string;
    page?: number;
    per_page?: number;
  }): Promise<PaginatedResponse<Transaction>> => {
    const response = await apiClient.get('/api/v1/exchange', { params });
    return response.data;
  },

  // Get transaction statistics
  getTransactionStatistics: async (params?: {
    from_date?: string;
    to_date?: string;
  }): Promise<TransactionStatistics> => {
    const response = await apiClient.get('/api/v1/exchange/statistics', { params });
    return response.data;
  },

  // Process expired transactions (admin only)
  processExpiredTransactions: async (): Promise<{ processed: number }> => {
    const response = await apiClient.post('/api/v1/admin/exchange/expired/process');
    return response.data;
  },
};
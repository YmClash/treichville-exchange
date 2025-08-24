import apiClient from './client';
import {
  WalletBalance,
  WalletTransaction,
  DepositDto,
  WithdrawDto,
  TransferDto,
  ReserveFundsDto,
  ReleaseFundsDto,
  WalletStatistics,
  WalletLimits,
} from '@/lib/types/wallet';
import { PaginatedResponse } from '@/lib/types/api';

export const walletApi = {
  // Get wallet balance (all currencies)
  getBalance: async (): Promise<WalletBalance[]> => {
    const response = await apiClient.get('/api/v1/wallet/balance');
    return response.data;
  },

  // Get balance for specific currency
  getBalanceByCurrency: async (currency: string): Promise<WalletBalance> => {
    const response = await apiClient.get(`/api/v1/wallet/balance/${currency}`);
    return response.data;
  },

  // Deposit funds
  deposit: async (data: DepositDto): Promise<WalletTransaction> => {
    const response = await apiClient.post('/api/v1/wallet/deposit', data);
    return response.data;
  },

  // Withdraw funds
  withdraw: async (data: WithdrawDto): Promise<WalletTransaction> => {
    const response = await apiClient.post('/api/v1/wallet/withdraw', data);
    return response.data;
  },

  // Transfer funds
  transfer: async (data: TransferDto): Promise<WalletTransaction> => {
    const response = await apiClient.post('/api/v1/wallet/transfer', data);
    return response.data;
  },

  // Reserve funds for transaction
  reserveFunds: async (data: ReserveFundsDto): Promise<{ reservation_id: string }> => {
    const response = await apiClient.post('/api/v1/wallet/reserve', data);
    return response.data;
  },

  // Release reserved funds
  releaseFunds: async (data: ReleaseFundsDto): Promise<void> => {
    await apiClient.post('/api/v1/wallet/release', data);
  },

  // Get wallet transactions history
  getWalletTransactions: async (params?: {
    currency?: string;
    type?: string;
    from_date?: string;
    to_date?: string;
    page?: number;
    per_page?: number;
  }): Promise<PaginatedResponse<WalletTransaction>> => {
    const response = await apiClient.get('/api/v1/wallet/transactions', { params });
    return response.data;
  },

  // Get wallet statistics
  getWalletStatistics: async (): Promise<WalletStatistics> => {
    const response = await apiClient.get('/api/v1/wallet/statistics');
    return response.data;
  },

  // Get wallet limits
  getWalletLimits: async (): Promise<WalletLimits> => {
    const response = await apiClient.get('/api/v1/wallet/limits');
    return response.data;
  },

  // Get pending operations
  getPendingOperations: async (): Promise<WalletTransaction[]> => {
    const response = await apiClient.get('/api/v1/wallet/pending-operations');
    return response.data;
  },

  // Validate operation
  validateOperation: async (operationId: string, code: string): Promise<void> => {
    await apiClient.post('/api/v1/wallet/validate-operation', {
      operation_id: operationId,
      validation_code: code,
    });
  },

  // Reconcile wallet (admin only)
  reconcileWallet: async (userId: string): Promise<void> => {
    await apiClient.post('/api/v1/admin/wallet/reconcile', { user_id: userId });
  },
};
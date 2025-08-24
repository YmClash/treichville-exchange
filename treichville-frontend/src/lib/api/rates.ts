import apiClient from './client';
import {
  Rate,
  CreateRateDto,
  UpdateRateDto,
  BulkUpdateRatesDto,
  RateQuote,
  MarketDepth,
  AggregatedRate,
} from '@/lib/types/rate';

export const ratesApi = {
  // Get public rates (no auth required)
  getPublicRates: async (params?: {
    from?: string;
    to?: string;
    type?: 'buy' | 'sell';
  }): Promise<Rate[]> => {
    const response = await apiClient.get('/api/v1/rates/public', { params });
    return response.data;
  },

  // Get best rate for a currency pair
  getBestRate: async (from: string, to: string): Promise<Rate> => {
    const response = await apiClient.get('/api/v1/rates/best', {
      params: { from, to },
    });
    return response.data;
  },

  // Get quote for exchange
  getQuote: async (
    from_currency: string,
    to_currency: string,
    amount: number
  ): Promise<RateQuote> => {
    const response = await apiClient.get('/api/v1/rates/quote', {
      params: { from_currency, to_currency, amount },
    });
    return response.data;
  },

  // Get market depth
  getMarketDepth: async (
    from: string,
    to: string
  ): Promise<MarketDepth> => {
    const response = await apiClient.get('/api/v1/rates/market-depth', {
      params: { from, to },
    });
    return response.data;
  },

  // Get aggregated rates
  getAggregatedRates: async (): Promise<AggregatedRate[]> => {
    const response = await apiClient.get('/api/v1/rates/aggregated');
    return response.data;
  },

  // Get rates (authenticated)
  getRates: async (params?: {
    changeur_id?: string;
    from?: string;
    to?: string;
    is_active?: boolean;
  }): Promise<Rate[]> => {
    const response = await apiClient.get('/api/v1/rates', { params });
    return response.data;
  },

  // Get changeur's own rates
  getChangeurRates: async (): Promise<Rate[]> => {
    const response = await apiClient.get('/api/v1/rates/changeur');
    return response.data;
  },

  // Create new rate (changeur only)
  createRate: async (data: CreateRateDto): Promise<Rate> => {
    const response = await apiClient.post('/api/v1/rates', data);
    return response.data;
  },

  // Update rate (changeur only)
  updateRate: async (id: string, data: UpdateRateDto): Promise<Rate> => {
    const response = await apiClient.put(`/api/v1/rates/${id}`, data);
    return response.data;
  },

  // Bulk update rates (changeur only)
  bulkUpdateRates: async (data: BulkUpdateRatesDto): Promise<Rate[]> => {
    const response = await apiClient.post('/api/v1/rates/bulk', data);
    return response.data;
  },

  // Delete rate (changeur only)
  deleteRate: async (id: string): Promise<void> => {
    await apiClient.delete(`/api/v1/rates/${id}`);
  },
};
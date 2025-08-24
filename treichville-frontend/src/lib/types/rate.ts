export type RateType = 'buy' | 'sell';

export interface Rate {
  id: string;
  changeur_id: string;
  changeur_name?: string;
  from_currency: string;
  to_currency: string;
  rate_type: RateType;
  rate: number;
  min_amount: number;
  max_amount: number;
  is_active: boolean;
  available_amount?: number;
  rating?: number;
  completion_time?: number;
  last_updated: string;
  created_at: string;
}

export interface CreateRateDto {
  from_currency: string;
  to_currency: string;
  rate_type: RateType;
  rate: number;
  min_amount: number;
  max_amount: number;
  available_amount?: number;
}

export interface UpdateRateDto extends Partial<CreateRateDto> {
  is_active?: boolean;
}

export interface BulkUpdateRatesDto {
  rates: Array<{
    id?: string;
    from_currency: string;
    to_currency: string;
    rate_type: RateType;
    rate: number;
    min_amount: number;
    max_amount: number;
  }>;
}

export interface RateQuote {
  from_currency: string;
  to_currency: string;
  amount_from: number;
  amount_to: number;
  rate: number;
  fee: number;
  total: number;
  expires_at: string;
}

export interface MarketDepth {
  currency_pair: string;
  bids: Array<{
    price: number;
    volume: number;
    changeur_count: number;
  }>;
  asks: Array<{
    price: number;
    volume: number;
    changeur_count: number;
  }>;
  spread: number;
  mid_price: number;
}

export interface AggregatedRate {
  from_currency: string;
  to_currency: string;
  best_buy_rate: number;
  best_sell_rate: number;
  average_rate: number;
  median_rate: number;
  total_volume: number;
  changeur_count: number;
  last_updated: string;
}
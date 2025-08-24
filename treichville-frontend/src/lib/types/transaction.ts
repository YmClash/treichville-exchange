export type TransactionType = 'exchange' | 'crypto_buy' | 'crypto_sell' | 'deposit' | 'withdrawal';
export type TransactionStatus = 'pending' | 'paid' | 'confirmed' | 'completed' | 'cancelled' | 'expired' | 'failed';
export type PaymentMethod = 'cash' | 'orange_money' | 'wave' | 'mtn_money' | 'moov_money' | 'bank_transfer';
export type UrgencyLevel = 'Low' | 'Normal' | 'High' | 'Critical';

export interface Transaction {
  id: string;
  reference: string;
  client_id: string;
  changeur_id?: string;
  type: TransactionType;
  status: TransactionStatus;
  from_currency: string;
  to_currency: string;
  amount_from: number;
  amount_to: number;
  rate_applied: number;
  fee: number;
  total_amount: number;
  payment_method: PaymentMethod;
  payment_reference?: string;
  payment_proof?: any;
  blockchain_tx_hash?: string;
  notes?: string;
  idempotency_key?: string;
  expires_at?: string;
  paid_at?: string;
  confirmed_at?: string;
  completed_at?: string;
  cancelled_at?: string;
  cancelled_reason?: string;
  metadata?: Record<string, any>;
  created_at: string;
  updated_at: string;
}

export interface InitiateExchangeDto {
  from_currency: string;
  to_currency: string;
  amount: number;
  operation: 'buy' | 'sell';
  payment_method: PaymentMethod;
  urgency_level: UrgencyLevel;
  notes?: string;
}

export interface ConfirmPaymentDto {
  transaction_id: string;
  payment_reference: string;
  payment_proof?: any;
}

export interface TransactionStatistics {
  total_transactions: number;
  total_volume: number;
  average_amount: number;
  success_rate: number;
  by_status: Record<TransactionStatus, number>;
  by_currency: Record<string, number>;
}
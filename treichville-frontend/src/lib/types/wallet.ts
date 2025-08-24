export interface WalletBalance {
  currency: string;
  available_balance: number;
  reserved_balance: number;
  total_balance: number;
  last_updated: string;
}

export interface WalletTransaction {
  id: string;
  wallet_id: string;
  type: 'deposit' | 'withdrawal' | 'transfer_in' | 'transfer_out' | 'reserve' | 'release';
  currency: string;
  amount: number;
  balance_after: number;
  reference?: string;
  description?: string;
  metadata?: Record<string, any>;
  created_at: string;
}

export interface DepositDto {
  currency: string;
  amount: number;
  payment_method: string;
  reference?: string;
}

export interface WithdrawDto {
  currency: string;
  amount: number;
  destination: string;
  payment_method: string;
}

export interface TransferDto {
  from_currency: string;
  to_currency: string;
  amount: number;
  recipient_id: string;
  notes?: string;
}

export interface ReserveFundsDto {
  currency: string;
  amount: number;
  transaction_id: string;
  duration_minutes?: number;
}

export interface ReleaseFundsDto {
  reservation_id: string;
}

export interface WalletStatistics {
  total_deposits: number;
  total_withdrawals: number;
  total_transfers: number;
  net_flow: number;
  by_currency: Record<string, {
    deposits: number;
    withdrawals: number;
    balance: number;
  }>;
}

export interface WalletLimits {
  daily_deposit_limit: number;
  daily_withdrawal_limit: number;
  monthly_deposit_limit: number;
  monthly_withdrawal_limit: number;
  daily_deposit_used: number;
  daily_withdrawal_used: number;
  monthly_deposit_used: number;
  monthly_withdrawal_used: number;
}
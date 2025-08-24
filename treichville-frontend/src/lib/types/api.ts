export interface ApiResponse<T = any> {
  success: boolean;
  data?: T;
  message?: string;
  error?: string;
  errors?: Record<string, string[]>;
}

export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  per_page: number;
  total_pages: number;
}

export interface ApiError {
  message: string;
  code?: string;
  status: number;
  details?: any;
}

export interface HealthStatus {
  status: 'healthy' | 'degraded' | 'unhealthy';
  timestamp: string;
  uptime: number;
  services: {
    database: boolean;
    redis: boolean;
    external_apis?: boolean;
  };
  version: string;
}

export interface Currency {
  code: string;
  name: string;
  symbol: string;
  decimal_places: number;
  is_crypto: boolean;
}

export const CURRENCIES: Currency[] = [
  { code: 'XOF', name: 'CFA Franc', symbol: 'CFA', decimal_places: 0, is_crypto: false },
  { code: 'EUR', name: 'Euro', symbol: '€', decimal_places: 2, is_crypto: false },
  { code: 'USD', name: 'US Dollar', symbol: '$', decimal_places: 2, is_crypto: false },
  { code: 'GBP', name: 'British Pound', symbol: '£', decimal_places: 2, is_crypto: false },
  { code: 'BTC', name: 'Bitcoin', symbol: '₿', decimal_places: 8, is_crypto: true },
  { code: 'USDT', name: 'Tether', symbol: 'USDT', decimal_places: 2, is_crypto: true },
];
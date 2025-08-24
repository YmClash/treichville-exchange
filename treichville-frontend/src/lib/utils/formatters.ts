import { format, formatDistance, formatRelative, parseISO } from 'date-fns';
import { fr, enUS } from 'date-fns/locale';
import numeral from 'numeral';

// Set locale for numeral
if (typeof window !== 'undefined') {
  numeral.register('locale', 'fr', {
    delimiters: {
      thousands: ' ',
      decimal: ','
    },
    abbreviations: {
      thousand: 'k',
      million: 'M',
      billion: 'B',
      trillion: 'T'
    },
    currency: {
      symbol: 'CFA'
    }
  });
  numeral.locale('fr');
}

// Currency formatting
export function formatCurrency(amount: number, currency: string = 'XOF'): string {
  const locale = 'fr-CI'; // French Côte d'Ivoire
  
  try {
    return new Intl.NumberFormat(locale, {
      style: 'currency',
      currency: currency,
      minimumFractionDigits: currency === 'XOF' ? 0 : 2,
      maximumFractionDigits: currency === 'XOF' ? 0 : 2,
    }).format(amount);
  } catch (error) {
    // Fallback for unsupported currencies
    const symbol = getCurrencySymbol(currency);
    const formatted = numeral(amount).format('0,0[.]00');
    return `${symbol} ${formatted}`;
  }
}

// Get currency symbol
export function getCurrencySymbol(currency: string): string {
  const symbols: Record<string, string> = {
    XOF: 'CFA',
    EUR: '€',
    USD: '$',
    GBP: '£',
    BTC: '₿',
    USDT: 'USDT',
  };
  return symbols[currency] || currency;
}

// Format number with separators
export function formatNumber(value: number, decimals: number = 2): string {
  return numeral(value).format(`0,0[.]${'0'.repeat(decimals)}`);
}

// Format compact number (1.2k, 3.4M, etc.)
export function formatCompactNumber(value: number): string {
  return numeral(value).format('0.0a').toUpperCase();
}

// Format percentage
export function formatPercentage(value: number, decimals: number = 2): string {
  return `${(value * 100).toFixed(decimals)}%`;
}

// Format phone number
export function formatPhone(phone: string): string {
  // Remove any non-digit characters
  const cleaned = phone.replace(/\D/g, '');
  
  // Format for Côte d'Ivoire numbers
  if (cleaned.startsWith('225')) {
    // +225 07 12 34 56 78
    return cleaned.replace(
      /(\d{3})(\d{2})(\d{2})(\d{2})(\d{2})(\d{2})/,
      '+$1 $2 $3 $4 $5 $6'
    );
  }
  
  // Format for local numbers (without country code)
  if (cleaned.length === 10) {
    // 07 12 34 56 78
    return cleaned.replace(
      /(\d{2})(\d{2})(\d{2})(\d{2})(\d{2})/,
      '$1 $2 $3 $4 $5'
    );
  }
  
  return phone;
}

// Date formatting
export function formatDate(date: string | Date, formatStr: string = 'dd/MM/yyyy'): string {
  const dateObj = typeof date === 'string' ? parseISO(date) : date;
  return format(dateObj, formatStr, { locale: fr });
}

// Format datetime
export function formatDateTime(date: string | Date): string {
  const dateObj = typeof date === 'string' ? parseISO(date) : date;
  return format(dateObj, 'dd/MM/yyyy HH:mm', { locale: fr });
}

// Format relative time (e.g., "il y a 5 minutes")
export function formatRelativeTime(date: string | Date): string {
  const dateObj = typeof date === 'string' ? parseISO(date) : date;
  return formatDistance(dateObj, new Date(), { 
    addSuffix: true,
    locale: fr 
  });
}

// Format time remaining
export function formatTimeRemaining(expiresAt: string | Date): string {
  const dateObj = typeof expiresAt === 'string' ? parseISO(expiresAt) : expiresAt;
  const now = new Date();
  
  if (dateObj <= now) {
    return 'Expiré';
  }
  
  const distance = formatDistance(dateObj, now, { locale: fr });
  return `Expire dans ${distance}`;
}

// Format transaction reference
export function formatReference(reference: string): string {
  // Format: TRX-YYYYMMDD-XXXX
  if (reference.startsWith('TRX-')) {
    const parts = reference.split('-');
    if (parts.length === 3) {
      return `${parts[0]}-****-${parts[2]}`;
    }
  }
  return reference;
}

// Format file size
export function formatFileSize(bytes: number): string {
  const sizes = ['Bytes', 'KB', 'MB', 'GB'];
  if (bytes === 0) return '0 Bytes';
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return Math.round(bytes / Math.pow(1024, i) * 100) / 100 + ' ' + sizes[i];
}

// Truncate text
export function truncateText(text: string, maxLength: number = 50): string {
  if (text.length <= maxLength) return text;
  return text.substring(0, maxLength) + '...';
}

// Format address (truncate middle for crypto addresses)
export function formatAddress(address: string, start: number = 6, end: number = 4): string {
  if (address.length <= start + end) return address;
  return `${address.slice(0, start)}...${address.slice(-end)}`;
}

// Format status badge color
export function getStatusColor(status: string): string {
  const colors: Record<string, string> = {
    pending: 'yellow',
    paid: 'blue',
    confirmed: 'indigo',
    completed: 'green',
    cancelled: 'gray',
    expired: 'red',
    failed: 'red',
  };
  return colors[status.toLowerCase()] || 'gray';
}

// Format urgency level color
export function getUrgencyColor(urgency: string): string {
  const colors: Record<string, string> = {
    Low: 'green',
    Normal: 'blue',
    High: 'orange',
    Critical: 'red',
  };
  return colors[urgency] || 'gray';
}
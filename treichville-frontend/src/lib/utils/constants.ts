// API Configuration
export const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080';
export const APP_URL = process.env.NEXT_PUBLIC_APP_URL || 'http://localhost:3000';
export const WS_URL = process.env.NEXT_PUBLIC_WS_URL || 'ws://localhost:8080/ws';

// App Configuration
export const APP_NAME = 'Treichville Exchange';
export const APP_DESCRIPTION = 'Plateforme de change de devises pour l\'Afrique de l\'Ouest';
export const APP_VERSION = '1.0.0';

// Currencies
export const CURRENCIES = [
  { code: 'XOF', name: 'Franc CFA', symbol: 'CFA', flag: '🇨🇮' },
  { code: 'EUR', name: 'Euro', symbol: '€', flag: '🇪🇺' },
  { code: 'USD', name: 'Dollar US', symbol: '$', flag: '🇺🇸' },
  { code: 'GBP', name: 'Livre Sterling', symbol: '£', flag: '🇬🇧' },
] as const;

export const CRYPTO_CURRENCIES = [
  { code: 'BTC', name: 'Bitcoin', symbol: '₿' },
  { code: 'USDT', name: 'Tether', symbol: 'USDT' },
] as const;

// Payment Methods
export const PAYMENT_METHODS = [
  { value: 'cash', label: 'Espèces', icon: '💵' },
  { value: 'orange_money', label: 'Orange Money', icon: '📱' },
  { value: 'wave', label: 'Wave', icon: '🌊' },
  { value: 'mtn_money', label: 'MTN Money', icon: '📲' },
  { value: 'moov_money', label: 'Moov Money', icon: '📞' },
] as const;

// Transaction Status
export const TRANSACTION_STATUS = {
  pending: { label: 'En attente', color: 'yellow' },
  paid: { label: 'Payé', color: 'blue' },
  confirmed: { label: 'Confirmé', color: 'indigo' },
  completed: { label: 'Complété', color: 'green' },
  cancelled: { label: 'Annulé', color: 'gray' },
  expired: { label: 'Expiré', color: 'red' },
  failed: { label: 'Échoué', color: 'red' },
} as const;

// Urgency Levels
export const URGENCY_LEVELS = [
  { value: 'Low', label: 'Faible (24h)', color: 'green' },
  { value: 'Normal', label: 'Normal (2-4h)', color: 'blue' },
  { value: 'High', label: 'Élevé (1h)', color: 'orange' },
  { value: 'Critical', label: 'Critique (15min)', color: 'red' },
] as const;

// User Roles
export const USER_ROLES = {
  client: { label: 'Client', icon: '👤' },
  changeur: { label: 'Changeur', icon: '💱' },
  admin: { label: 'Admin', icon: '👨‍💼' },
  super_admin: { label: 'Super Admin', icon: '🔐' },
} as const;

// KYC Levels
export const KYC_LEVELS = {
  none: { label: 'Non vérifié', color: 'gray' },
  level0: { label: 'Niveau 0', color: 'yellow' },
  level1: { label: 'Niveau 1', color: 'blue' },
  level2: { label: 'Niveau 2', color: 'green' },
} as const;

// Limits
export const DEFAULT_LIMITS = {
  client: {
    daily: 50000,
    monthly: 500000,
  },
  changeur: {
    daily: 10000000,
    monthly: 100000000,
  },
} as const;

// Pagination
export const PAGINATION = {
  defaultPageSize: 10,
  pageSizeOptions: [10, 20, 50, 100],
} as const;

// Date Formats
export const DATE_FORMATS = {
  display: 'dd/MM/yyyy',
  displayTime: 'dd/MM/yyyy HH:mm',
  api: 'yyyy-MM-dd',
  apiTime: "yyyy-MM-dd'T'HH:mm:ss",
} as const;

// Validation Rules
export const VALIDATION = {
  phone: {
    minLength: 10,
    maxLength: 15,
    pattern: /^(\+225)?[0-9]{10}$/,
  },
  password: {
    minLength: 8,
    maxLength: 100,
    pattern: /^(?=.*[a-z])(?=.*[A-Z])(?=.*\d)[a-zA-Z\d@$!%*?&]{8,}$/,
  },
  name: {
    minLength: 2,
    maxLength: 100,
  },
  amount: {
    min: 1,
    max: 1000000000,
  },
} as const;

// Time Intervals (in milliseconds)
export const INTERVALS = {
  rateRefresh: 30000, // 30 seconds
  transactionExpiry: 900000, // 15 minutes
  sessionTimeout: 1800000, // 30 minutes
  tokenRefresh: 840000, // 14 minutes (before 15 min expiry)
} as const;

// Routes
export const ROUTES = {
  home: '/',
  login: '/login',
  register: '/register',
  dashboard: '/dashboard',
  exchange: '/exchange',
  wallet: '/wallet',
  rates: '/rates',
  transactions: '/transactions',
  profile: '/profile',
  // Changeur routes
  myRates: '/my-rates',
  analytics: '/analytics',
  // Admin routes
  adminUsers: '/admin/users',
  adminSettings: '/admin/settings',
} as const;

// Storage Keys
export const STORAGE_KEYS = {
  accessToken: 'access_token',
  refreshToken: 'refresh_token',
  theme: 'theme',
  language: 'language',
  lastActivity: 'last_activity',
} as const;

// Error Messages
export const ERROR_MESSAGES = {
  network: 'Erreur de connexion. Veuillez vérifier votre connexion internet.',
  unauthorized: 'Session expirée. Veuillez vous reconnecter.',
  forbidden: 'Vous n\'avez pas les permissions nécessaires.',
  notFound: 'La ressource demandée n\'existe pas.',
  serverError: 'Une erreur serveur est survenue. Veuillez réessayer.',
  validation: 'Veuillez vérifier les informations saisies.',
  generic: 'Une erreur est survenue. Veuillez réessayer.',
} as const;

// Success Messages
export const SUCCESS_MESSAGES = {
  login: 'Connexion réussie !',
  register: 'Inscription réussie ! Vous pouvez maintenant vous connecter.',
  logout: 'Déconnexion réussie.',
  profileUpdate: 'Profil mis à jour avec succès.',
  passwordChange: 'Mot de passe modifié avec succès.',
  transactionInitiated: 'Transaction initiée avec succès.',
  transactionCompleted: 'Transaction complétée avec succès.',
  rateCreated: 'Taux créé avec succès.',
  rateUpdated: 'Taux mis à jour avec succès.',
  rateDeleted: 'Taux supprimé avec succès.',
} as const;
import { z } from 'zod';

// Phone validation for Côte d'Ivoire
// Accepts: 0712345678, +2250712345678, 07 12 34 56 78
const phoneRegex = /^(\+225)?0?[0-9]{9,10}$/;

// Password validation (min 8 chars, 1 uppercase, 1 lowercase, 1 number)
const passwordRegex = /^(?=.*[a-z])(?=.*[A-Z])(?=.*\d)[a-zA-Z\d@$!%*?&]{8,}$/;

// Common schemas
export const phoneSchema = z
  .string()
  .transform((val) => val.replace(/[\s-]/g, '')) // Remove spaces and dashes
  .refine((val) => val.length >= 10, {
    message: 'Le numéro de téléphone doit contenir au moins 10 chiffres'
  })
  .refine((val) => phoneRegex.test(val), {
    message: 'Format de numéro invalide'
  });

export const passwordSchema = z
  .string()
  .min(8, 'Le mot de passe doit contenir au moins 8 caractères')
  .regex(
    passwordRegex,
    'Le mot de passe doit contenir au moins une majuscule, une minuscule et un chiffre'
  );

export const emailSchema = z
  .string()
  .email('Email invalide')
  .optional()
  .or(z.literal(''));

// Auth schemas
export const loginSchema = z.object({
  phone: phoneSchema,
  password: z.string().min(1, 'Le mot de passe est requis'),
});

export const registerSchema = z
  .object({
    phone: phoneSchema,
    email: emailSchema,
    name: z
      .string()
      .min(2, 'Le nom doit contenir au moins 2 caractères')
      .max(100, 'Le nom ne peut pas dépasser 100 caractères'),
    password: passwordSchema,
    password_confirmation: z.string(),
    role: z.enum(['client', 'changeur']),
  })
  .refine((data) => data.password === data.password_confirmation, {
    message: 'Les mots de passe ne correspondent pas',
    path: ['password_confirmation'],
  });

export const changePasswordSchema = z
  .object({
    current_password: z.string().min(1, 'Le mot de passe actuel est requis'),
    new_password: passwordSchema,
    new_password_confirmation: z.string(),
  })
  .refine((data) => data.new_password === data.new_password_confirmation, {
    message: 'Les mots de passe ne correspondent pas',
    path: ['new_password_confirmation'],
  })
  .refine((data) => data.current_password !== data.new_password, {
    message: 'Le nouveau mot de passe doit être différent de l\'ancien',
    path: ['new_password'],
  });

// Exchange schemas
export const initiateExchangeSchema = z.object({
  from_currency: z.enum(['EUR', 'USD', 'XOF', 'GBP']),
  to_currency: z.enum(['EUR', 'USD', 'XOF', 'GBP']),
  amount: z
    .number()
    .positive('Le montant doit être positif')
    .min(1, 'Le montant minimum est 1'),
  operation: z.enum(['buy', 'sell']),
  payment_method: z.enum(['cash', 'orange_money', 'wave', 'mtn_money', 'moov_money']),
  urgency_level: z.enum(['Low', 'Normal', 'High', 'Critical']),
  notes: z.string().optional(),
});

export const confirmPaymentSchema = z.object({
  transaction_id: z.string().uuid('ID de transaction invalide'),
  payment_reference: z.string().min(1, 'La référence de paiement est requise'),
  payment_proof: z.any().optional(),
});

// Rate schemas
export const createRateSchema = z.object({
  from_currency: z.string().min(3).max(3),
  to_currency: z.string().min(3).max(3),
  rate_type: z.enum(['buy', 'sell']),
  rate: z
    .number()
    .positive('Le taux doit être positif')
    .min(0.0001, 'Le taux minimum est 0.0001'),
  min_amount: z
    .number()
    .positive('Le montant minimum doit être positif')
    .min(1),
  max_amount: z
    .number()
    .positive('Le montant maximum doit être positif'),
  available_amount: z.number().positive().optional(),
}).refine((data) => data.max_amount > data.min_amount, {
  message: 'Le montant maximum doit être supérieur au montant minimum',
  path: ['max_amount'],
});

export const updateRateSchema = createRateSchema.partial().extend({
  is_active: z.boolean().optional(),
});

// Wallet schemas
export const depositSchema = z.object({
  currency: z.string().min(3).max(3),
  amount: z
    .number()
    .positive('Le montant doit être positif')
    .min(1, 'Le montant minimum est 1'),
  payment_method: z.string().min(1, 'La méthode de paiement est requise'),
  reference: z.string().optional(),
});

export const withdrawSchema = z.object({
  currency: z.string().min(3).max(3),
  amount: z
    .number()
    .positive('Le montant doit être positif')
    .min(1, 'Le montant minimum est 1'),
  destination: z.string().min(1, 'La destination est requise'),
  payment_method: z.string().min(1, 'La méthode de paiement est requise'),
});

export const transferSchema = z.object({
  from_currency: z.string().min(3).max(3),
  to_currency: z.string().min(3).max(3),
  amount: z
    .number()
    .positive('Le montant doit être positif')
    .min(1, 'Le montant minimum est 1'),
  recipient_id: z.string().uuid('ID du destinataire invalide'),
  notes: z.string().optional(),
});

// Profile schemas
export const updateProfileSchema = z.object({
  name: z
    .string()
    .min(2, 'Le nom doit contenir au moins 2 caractères')
    .max(100, 'Le nom ne peut pas dépasser 100 caractères')
    .optional(),
  email: emailSchema,
  phone: phoneSchema.optional(),
});

// 2FA schemas
export const twoFactorCodeSchema = z.object({
  code: z
    .string()
    .length(6, 'Le code doit contenir 6 chiffres')
    .regex(/^\d{6}$/, 'Le code doit contenir uniquement des chiffres'),
});

// Types exports
export type LoginInput = z.infer<typeof loginSchema>;
export type RegisterInput = z.infer<typeof registerSchema>;
export type ChangePasswordInput = z.infer<typeof changePasswordSchema>;
export type InitiateExchangeInput = z.infer<typeof initiateExchangeSchema>;
export type ConfirmPaymentInput = z.infer<typeof confirmPaymentSchema>;
export type CreateRateInput = z.infer<typeof createRateSchema>;
export type UpdateRateInput = z.infer<typeof updateRateSchema>;
export type DepositInput = z.infer<typeof depositSchema>;
export type WithdrawInput = z.infer<typeof withdrawSchema>;
export type TransferInput = z.infer<typeof transferSchema>;
export type UpdateProfileInput = z.infer<typeof updateProfileSchema>;
export type TwoFactorCodeInput = z.infer<typeof twoFactorCodeSchema>;
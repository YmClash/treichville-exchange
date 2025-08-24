export type UserRole = 'client' | 'changeur' | 'admin' | 'super_admin';
export type KycLevel = 'none' | 'level0' | 'level1' | 'level2';

export interface User {
  id: string;
  phone: string;
  email?: string;
  name: string;
  role: UserRole;
  kyc_status: KycLevel;
  daily_limit: number;
  monthly_limit: number;
  is_active: boolean;
  is_verified: boolean;
  two_fa_enabled: boolean;
  last_login_at?: string;
  metadata?: Record<string, any>;
  created_at: string;
  updated_at: string;
}

export interface LoginDto {
  phone: string;
  password: string;
}

export interface RegisterDto {
  phone: string;
  email?: string;
  name: string;
  password: string;
  password_confirmation: string;
  role: 'client' | 'changeur';
}

export interface AuthResponse {
  access_token: string;
  refresh_token: string;
  user: User;
}

export interface UpdateProfileDto {
  name?: string;
  email?: string;
  phone?: string;
}

export interface ChangePasswordDto {
  current_password: string;
  new_password: string;
  new_password_confirmation: string;
}
import apiClient, { setTokens, clearTokens } from './client';
import { 
  User, 
  LoginDto, 
  RegisterDto, 
  AuthResponse, 
  UpdateProfileDto,
  ChangePasswordDto 
} from '@/lib/types/user';

export const authApi = {
  // Register new user
  register: async (data: RegisterDto): Promise<AuthResponse> => {
    const response = await apiClient.post('/api/v1/auth/register', data);
    return response.data;
  },

  // Login
  login: async (data: LoginDto): Promise<AuthResponse> => {
    const response = await apiClient.post('/api/v1/auth/login', data);
    const authData = response.data;
    
    // Store tokens
    setTokens(authData.access_token, authData.refresh_token);
    
    return authData;
  },

  // Logout
  logout: async (): Promise<void> => {
    try {
      await apiClient.post('/api/v1/auth/logout');
    } finally {
      clearTokens();
      window.location.href = '/login';
    }
  },

  // Get current user
  getCurrentUser: async (): Promise<User> => {
    const response = await apiClient.get('/api/v1/auth/me');
    return response.data;
  },

  // Update profile
  updateProfile: async (data: UpdateProfileDto): Promise<User> => {
    const response = await apiClient.put('/api/v1/auth/update-profile', data);
    return response.data;
  },

  // Change password
  changePassword: async (data: ChangePasswordDto): Promise<void> => {
    await apiClient.post('/api/v1/auth/change-password', data);
  },

  // Verify email
  verifyEmail: async (token: string): Promise<void> => {
    await apiClient.get(`/api/v1/auth/verify-email/${token}`);
  },

  // Forgot password
  forgotPassword: async (phone: string): Promise<void> => {
    await apiClient.post('/api/v1/auth/forgot-password', { phone });
  },

  // Reset password
  resetPassword: async (token: string, newPassword: string): Promise<void> => {
    await apiClient.post('/api/v1/auth/reset-password', {
      token,
      new_password: newPassword,
    });
  },

  // Enable 2FA
  enable2FA: async (): Promise<{ secret: string; qr_code: string }> => {
    const response = await apiClient.post('/api/v1/auth/enable-2fa');
    return response.data;
  },

  // Verify 2FA
  verify2FA: async (code: string): Promise<void> => {
    await apiClient.post('/api/v1/auth/verify-2fa', { code });
  },

  // Disable 2FA
  disable2FA: async (code: string): Promise<void> => {
    await apiClient.post('/api/v1/auth/disable-2fa', { code });
  },
};
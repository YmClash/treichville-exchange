import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';
import { User } from '@/lib/types/user';
import { jwtDecode } from 'jwt-decode';

interface JwtPayload {
  user_id: string;
  phone: string;
  role: string;
  exp: number;
  iat: number;
}

interface AuthState {
  user: User | null;
  accessToken: string | null;
  refreshToken: string | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  
  // Actions
  setUser: (user: User) => void;
  setTokens: (accessToken: string, refreshToken: string) => void;
  setLoading: (loading: boolean) => void;
  clearAuth: () => void;
  isTokenExpired: () => boolean;
  getUserRole: () => string | null;
  hasRole: (role: string) => boolean;
  isAdmin: () => boolean;
  isChangeur: () => boolean;
  isClient: () => boolean;
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set, get) => ({
      user: null,
      accessToken: null,
      refreshToken: null,
      isAuthenticated: false,
      isLoading: false,

      setUser: (user) => 
        set({ 
          user, 
          isAuthenticated: true 
        }),
      
      setTokens: (accessToken, refreshToken) => {
        // Store tokens in localStorage as well for API client
        if (typeof window !== 'undefined') {
          localStorage.setItem('access_token', accessToken);
          localStorage.setItem('refresh_token', refreshToken);
        }
        
        set({
          accessToken,
          refreshToken,
          isAuthenticated: true,
        });
      },

      setLoading: (loading) => set({ isLoading: loading }),
      
      clearAuth: () => {
        // Clear tokens from localStorage
        if (typeof window !== 'undefined') {
          localStorage.removeItem('access_token');
          localStorage.removeItem('refresh_token');
        }
        
        set({
          user: null,
          accessToken: null,
          refreshToken: null,
          isAuthenticated: false,
        });
      },
      
      isTokenExpired: () => {
        const token = get().accessToken;
        if (!token) return true;
        
        try {
          const decoded = jwtDecode<JwtPayload>(token);
          return decoded.exp * 1000 < Date.now();
        } catch {
          return true;
        }
      },

      getUserRole: () => {
        const user = get().user;
        return user?.role || null;
      },

      hasRole: (role) => {
        const user = get().user;
        return user?.role === role;
      },

      isAdmin: () => {
        const user = get().user;
        return user?.role === 'admin' || user?.role === 'super_admin';
      },

      isChangeur: () => {
        const user = get().user;
        return user?.role === 'changeur';
      },

      isClient: () => {
        const user = get().user;
        return user?.role === 'client';
      },
    }),
    {
      name: 'auth-storage',
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({
        user: state.user,
        accessToken: state.accessToken,
        refreshToken: state.refreshToken,
        isAuthenticated: state.isAuthenticated,
      }),
    }
  )
);
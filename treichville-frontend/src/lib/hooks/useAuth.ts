import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useRouter } from 'next/navigation';
import { useAuthStore } from '@/lib/stores/authStore';
import { authApi } from '@/lib/api/auth';
import { LoginInput, RegisterInput } from '@/lib/utils/validators';
import { toast } from '@/lib/hooks/useToast';
import { ROUTES, SUCCESS_MESSAGES, ERROR_MESSAGES } from '@/lib/utils/constants';

export function useAuth() {
  const router = useRouter();
  const queryClient = useQueryClient();
  const { 
    setUser, 
    setTokens, 
    clearAuth, 
    user,
    isAuthenticated,
    getUserRole,
    hasRole,
    isAdmin,
    isChangeur,
    isClient
  } = useAuthStore();

  // Login mutation
  const loginMutation = useMutation({
    mutationFn: authApi.login,
    onSuccess: (data) => {
      setTokens(data.access_token, data.refresh_token);
      setUser(data.user);
      
      // Redirect based on role
      if (data.user.role === 'admin' || data.user.role === 'super_admin') {
        router.push(ROUTES.adminUsers);
      } else if (data.user.role === 'changeur') {
        router.push(ROUTES.myRates);
      } else {
        router.push(ROUTES.exchange);
      }
      
      toast({
        title: SUCCESS_MESSAGES.login,
        description: `Bienvenue ${data.user.name}`,
      });
    },
    onError: (error: any) => {
      toast({
        title: 'Erreur de connexion',
        description: error.message || ERROR_MESSAGES.generic,
        variant: 'destructive',
      });
    },
  });

  // Register mutation
  const registerMutation = useMutation({
    mutationFn: authApi.register,
    onSuccess: (data) => {
      toast({
        title: SUCCESS_MESSAGES.register,
        variant: 'success',
      });
      router.push(ROUTES.login);
    },
    onError: (error: any) => {
      toast({
        title: 'Erreur d\'inscription',
        description: error.message || ERROR_MESSAGES.generic,
        variant: 'destructive',
      });
    },
  });

  // Logout mutation
  const logoutMutation = useMutation({
    mutationFn: authApi.logout,
    onSuccess: () => {
      clearAuth();
      queryClient.clear();
      router.push(ROUTES.login);
      toast({
        title: SUCCESS_MESSAGES.logout,
      });
    },
    onError: () => {
      // Clear auth even if logout fails
      clearAuth();
      queryClient.clear();
      router.push(ROUTES.login);
    },
  });

  // Get current user query
  const { data: currentUser, isLoading: isLoadingUser } = useQuery({
    queryKey: ['currentUser'],
    queryFn: authApi.getCurrentUser,
    enabled: isAuthenticated && !user,
    staleTime: 5 * 60 * 1000, // 5 minutes
  });

  // Update user in store when fetched
  if (currentUser && !user) {
    setUser(currentUser);
  }

  return {
    // State
    user,
    isAuthenticated,
    isLoadingUser,
    
    // Role checks
    getUserRole,
    hasRole,
    isAdmin,
    isChangeur,
    isClient,
    
    // Actions
    login: (data: LoginInput) => loginMutation.mutate(data),
    register: (data: RegisterInput) => registerMutation.mutate(data),
    logout: () => logoutMutation.mutate(),
    
    // Loading states
    isLoggingIn: loginMutation.isPending,
    isRegistering: registerMutation.isPending,
    isLoggingOut: logoutMutation.isPending,
  };
}

// Hook for protected routes
export function useRequireAuth(requiredRole?: string) {
  const router = useRouter();
  const { isAuthenticated, user, hasRole } = useAuth();

  if (!isAuthenticated) {
    router.push(ROUTES.login);
    return { isAuthorized: false };
  }

  if (requiredRole && !hasRole(requiredRole)) {
    router.push(ROUTES.dashboard);
    return { isAuthorized: false };
  }

  return { isAuthorized: true, user };
}
'use client';

import { useEffect } from 'react';
import { useRouter } from 'next/navigation';
import { useAuthStore } from '@/lib/stores/authStore';
import { ROUTES } from '@/lib/utils/constants';

export default function HomePage() {
  const router = useRouter();
  const { isAuthenticated, getUserRole } = useAuthStore();

  useEffect(() => {
    if (isAuthenticated) {
      const role = getUserRole();
      if (role === 'admin' || role === 'super_admin') {
        router.push(ROUTES.adminUsers);
      } else if (role === 'changeur') {
        router.push(ROUTES.myRates);
      } else {
        router.push(ROUTES.exchange);
      }
    } else {
      router.push(ROUTES.login);
    }
  }, [isAuthenticated, getUserRole, router]);

  return (
    <div className="min-h-screen flex items-center justify-center bg-gradient-to-br from-orange-50 to-green-50">
      <div className="text-center">
        <div className="w-20 h-20 bg-gradient-to-br from-orange-500 to-green-600 rounded-full flex items-center justify-center mx-auto mb-4 animate-pulse">
          <span className="text-white text-3xl font-bold">TE</span>
        </div>
        <p className="text-gray-600">Redirection...</p>
      </div>
    </div>
  );
}
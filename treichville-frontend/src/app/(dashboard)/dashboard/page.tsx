'use client';

import { useQuery } from '@tanstack/react-query';
import { 
  TrendingUp, TrendingDown, DollarSign, Clock, 
  ArrowUpRight, ArrowDownRight, Activity, Users,
  Wallet as WalletIcon
} from 'lucide-react';
import Link from 'next/link';
import { useAuthStore } from '@/lib/stores/authStore';
import { walletApi } from '@/lib/api/wallet';
import { exchangeApi } from '@/lib/api/exchange';
import { ratesApi } from '@/lib/api/rates';
import { formatCurrency, formatRelativeTime } from '@/lib/utils/formatters';

export default function DashboardPage() {
  const { user } = useAuthStore();

  // Fetch wallet balance
  const { data: walletBalance } = useQuery({
    queryKey: ['walletBalance'],
    queryFn: walletApi.getBalance,
  });

  // Fetch recent transactions
  const { data: recentTransactions } = useQuery({
    queryKey: ['recentTransactions'],
    queryFn: () => exchangeApi.getUserTransactions({ per_page: 5 }),
  });

  // Fetch transaction statistics
  const { data: stats } = useQuery({
    queryKey: ['transactionStats'],
    queryFn: exchangeApi.getTransactionStatistics,
  });

  // Fetch best rates
  const { data: bestRates } = useQuery({
    queryKey: ['bestRates'],
    queryFn: ratesApi.getAggregatedRates,
  });

  // Calculate total balance
  const totalBalance = walletBalance?.reduce((acc, balance) => {
    // Convert to XOF for total
    const rate = balance.currency === 'XOF' ? 1 : 
                  balance.currency === 'EUR' ? 655.96 : 
                  balance.currency === 'USD' ? 598.45 : 1;
    return acc + (balance.total_balance * rate);
  }, 0) || 0;

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      {/* Welcome Header */}
      <div className="mb-8">
        <h1 className="text-3xl font-bold text-gray-900">
          Bonjour, {user?.name} 👋
        </h1>
        <p className="text-gray-600 mt-1">
          Voici un aperçu de votre activité
        </p>
      </div>

      {/* Stats Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
        {/* Total Balance */}
        <div className="bg-white rounded-xl shadow-sm p-6">
          <div className="flex items-center justify-between mb-4">
            <div className="p-2 bg-orange-100 rounded-lg">
              <WalletIcon className="h-6 w-6 text-orange-600" />
            </div>
            <span className="text-xs text-green-600 font-medium flex items-center">
              <TrendingUp className="h-3 w-3 mr-1" />
              +12.5%
            </span>
          </div>
          <p className="text-sm text-gray-600">Solde Total</p>
          <p className="text-2xl font-bold text-gray-900">
            {formatCurrency(totalBalance, 'XOF')}
          </p>
        </div>

        {/* Total Transactions */}
        <div className="bg-white rounded-xl shadow-sm p-6">
          <div className="flex items-center justify-between mb-4">
            <div className="p-2 bg-green-100 rounded-lg">
              <Activity className="h-6 w-6 text-green-600" />
            </div>
            <span className="text-xs text-gray-500">Ce mois</span>
          </div>
          <p className="text-sm text-gray-600">Transactions</p>
          <p className="text-2xl font-bold text-gray-900">
            {stats?.total_transactions || 0}
          </p>
        </div>

        {/* Success Rate */}
        <div className="bg-white rounded-xl shadow-sm p-6">
          <div className="flex items-center justify-between mb-4">
            <div className="p-2 bg-blue-100 rounded-lg">
              <TrendingUp className="h-6 w-6 text-blue-600" />
            </div>
            <span className="text-xs text-gray-500">Taux</span>
          </div>
          <p className="text-sm text-gray-600">Taux de Succès</p>
          <p className="text-2xl font-bold text-gray-900">
            {stats?.success_rate || 0}%
          </p>
        </div>

        {/* Volume */}
        <div className="bg-white rounded-xl shadow-sm p-6">
          <div className="flex items-center justify-between mb-4">
            <div className="p-2 bg-purple-100 rounded-lg">
              <DollarSign className="h-6 w-6 text-purple-600" />
            </div>
            <span className="text-xs text-gray-500">Total</span>
          </div>
          <p className="text-sm text-gray-600">Volume Échangé</p>
          <p className="text-2xl font-bold text-gray-900">
            {formatCurrency(stats?.total_volume || 0, 'XOF')}
          </p>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
        {/* Recent Transactions */}
        <div className="lg:col-span-2 bg-white rounded-xl shadow-sm p-6">
          <div className="flex items-center justify-between mb-6">
            <h2 className="text-lg font-semibold text-gray-900">
              Transactions Récentes
            </h2>
            <Link
              href="/transactions"
              className="text-sm text-orange-600 hover:text-orange-700 font-medium"
            >
              Voir tout →
            </Link>
          </div>

          <div className="space-y-4">
            {recentTransactions?.items?.length === 0 ? (
              <p className="text-gray-500 text-center py-8">
                Aucune transaction récente
              </p>
            ) : (
              recentTransactions?.items?.map((transaction) => (
                <div
                  key={transaction.id}
                  className="flex items-center justify-between p-4 border border-gray-200 rounded-lg hover:bg-gray-50 transition-colors"
                >
                  <div className="flex items-center space-x-4">
                    <div className={`p-2 rounded-lg ${
                      transaction.type === 'exchange' ? 'bg-blue-100' :
                      transaction.type === 'deposit' ? 'bg-green-100' :
                      'bg-red-100'
                    }`}>
                      {transaction.type === 'exchange' ? (
                        <ArrowUpRight className="h-5 w-5 text-blue-600" />
                      ) : transaction.type === 'deposit' ? (
                        <ArrowDownRight className="h-5 w-5 text-green-600" />
                      ) : (
                        <ArrowUpRight className="h-5 w-5 text-red-600" />
                      )}
                    </div>
                    <div>
                      <p className="font-medium text-gray-900">
                        {transaction.from_currency} → {transaction.to_currency}
                      </p>
                      <p className="text-sm text-gray-500">
                        {formatRelativeTime(transaction.created_at)}
                      </p>
                    </div>
                  </div>
                  <div className="text-right">
                    <p className="font-semibold text-gray-900">
                      {formatCurrency(transaction.amount_from, transaction.from_currency)}
                    </p>
                    <span className={`text-xs px-2 py-1 rounded-full ${
                      transaction.status === 'completed' ? 'bg-green-100 text-green-700' :
                      transaction.status === 'pending' ? 'bg-yellow-100 text-yellow-700' :
                      'bg-red-100 text-red-700'
                    }`}>
                      {transaction.status}
                    </span>
                  </div>
                </div>
              ))
            )}
          </div>
        </div>

        {/* Quick Actions & Rates */}
        <div className="space-y-6">
          {/* Quick Actions */}
          <div className="bg-white rounded-xl shadow-sm p-6">
            <h2 className="text-lg font-semibold text-gray-900 mb-4">
              Actions Rapides
            </h2>
            <div className="space-y-3">
              <Link
                href="/exchange"
                className="flex items-center justify-between p-3 bg-gradient-to-r from-orange-500 to-orange-600 text-white rounded-lg hover:from-orange-600 hover:to-orange-700 transition-all"
              >
                <span className="font-medium">Échanger</span>
                <ArrowUpRight className="h-5 w-5" />
              </Link>
              <Link
                href="/wallet"
                className="flex items-center justify-between p-3 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors"
              >
                <span className="font-medium">Déposer</span>
                <ArrowDownRight className="h-5 w-5" />
              </Link>
              <Link
                href="/transactions"
                className="flex items-center justify-between p-3 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors"
              >
                <span className="font-medium">Historique</span>
                <Clock className="h-5 w-5" />
              </Link>
            </div>
          </div>

          {/* Live Rates */}
          <div className="bg-white rounded-xl shadow-sm p-6">
            <h2 className="text-lg font-semibold text-gray-900 mb-4">
              Taux Actuels
            </h2>
            <div className="space-y-3">
              {bestRates?.slice(0, 3).map((rate) => (
                <div key={`${rate.from_currency}-${rate.to_currency}`} className="flex justify-between items-center">
                  <span className="text-gray-600">
                    {rate.from_currency}/{rate.to_currency}
                  </span>
                  <div className="text-right">
                    <p className="font-semibold">
                      {rate.average_rate.toFixed(2)}
                    </p>
                    <p className="text-xs text-green-600">
                      <TrendingUp className="inline h-3 w-3" /> +0.2%
                    </p>
                  </div>
                </div>
              ))}
            </div>
            <Link
              href="/rates"
              className="block mt-4 text-center text-sm text-orange-600 hover:text-orange-700 font-medium"
            >
              Voir tous les taux →
            </Link>
          </div>
        </div>
      </div>
    </div>
  );
}
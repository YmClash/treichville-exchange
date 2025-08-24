'use client';

import { useState } from 'react';
import { useQuery, useMutation } from '@tanstack/react-query';
import { 
  Wallet as WalletIcon, Plus, ArrowUpRight, ArrowDownRight,
  Send, Download, RefreshCw, Shield, Info, Loader2
} from 'lucide-react';
import { walletApi } from '@/lib/api/wallet';
import { formatCurrency, formatDateTime } from '@/lib/utils/formatters';
import { CURRENCIES } from '@/lib/utils/constants';
import { toast } from '@/lib/hooks/useToast';

export default function WalletPage() {
  const [selectedCurrency, setSelectedCurrency] = useState<string>('all');
  const [isDepositModalOpen, setIsDepositModalOpen] = useState(false);
  const [isWithdrawModalOpen, setIsWithdrawModalOpen] = useState(false);
  const [isTransferModalOpen, setIsTransferModalOpen] = useState(false);

  // Fetch wallet balances
  const { data: balances, refetch: refetchBalances } = useQuery({
    queryKey: ['walletBalances'],
    queryFn: walletApi.getBalance,
    refetchInterval: 30000, // Refresh every 30 seconds
  });

  // Fetch wallet transactions
  const { data: transactions } = useQuery({
    queryKey: ['walletTransactions', selectedCurrency],
    queryFn: () => walletApi.getWalletTransactions({
      currency: selectedCurrency === 'all' ? undefined : selectedCurrency,
      per_page: 20,
    }),
  });

  // Fetch wallet statistics
  const { data: stats } = useQuery({
    queryKey: ['walletStats'],
    queryFn: walletApi.getWalletStatistics,
  });

  // Calculate total balance in XOF
  const totalBalanceXOF = balances?.reduce((acc, balance) => {
    const rate = balance.currency === 'XOF' ? 1 : 
                  balance.currency === 'EUR' ? 655.96 : 
                  balance.currency === 'USD' ? 598.45 : 1;
    return acc + (balance.total_balance * rate);
  }, 0) || 0;

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      {/* Header */}
      <div className="mb-8">
        <h1 className="text-3xl font-bold text-gray-900">Mon Portefeuille</h1>
        <p className="text-gray-600 mt-1">
          Gérez vos fonds en toute sécurité
        </p>
      </div>

      {/* Total Balance Card */}
      <div className="bg-gradient-to-r from-orange-500 to-orange-600 rounded-xl shadow-lg p-6 mb-8 text-white">
        <div className="flex items-center justify-between mb-4">
          <div>
            <p className="text-orange-100 text-sm">Solde Total</p>
            <p className="text-4xl font-bold">
              {formatCurrency(totalBalanceXOF, 'XOF')}
            </p>
          </div>
          <div className="p-4 bg-white/20 rounded-full">
            <WalletIcon className="h-8 w-8" />
          </div>
        </div>
        <div className="flex items-center space-x-4">
          <button
            onClick={() => setIsDepositModalOpen(true)}
            className="flex items-center px-4 py-2 bg-white text-orange-600 rounded-lg font-medium hover:bg-orange-50 transition-colors"
          >
            <Plus className="h-4 w-4 mr-2" />
            Déposer
          </button>
          <button
            onClick={() => setIsWithdrawModalOpen(true)}
            className="flex items-center px-4 py-2 bg-white/20 text-white rounded-lg font-medium hover:bg-white/30 transition-colors"
          >
            <ArrowUpRight className="h-4 w-4 mr-2" />
            Retirer
          </button>
          <button
            onClick={() => setIsTransferModalOpen(true)}
            className="flex items-center px-4 py-2 bg-white/20 text-white rounded-lg font-medium hover:bg-white/30 transition-colors"
          >
            <Send className="h-4 w-4 mr-2" />
            Transférer
          </button>
        </div>
      </div>

      {/* Currency Balances */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
        {balances?.map((balance) => {
          const currency = CURRENCIES.find(c => c.code === balance.currency);
          return (
            <div key={balance.currency} className="bg-white rounded-xl shadow-sm p-6">
              <div className="flex items-center justify-between mb-4">
                <div className="flex items-center space-x-2">
                  <span className="text-2xl">{currency?.flag}</span>
                  <span className="font-semibold text-gray-900">
                    {balance.currency}
                  </span>
                </div>
                {balance.reserved_balance > 0 && (
                  <div className="p-1 bg-yellow-100 rounded">
                    <Shield className="h-4 w-4 text-yellow-600" />
                  </div>
                )}
              </div>
              <div className="space-y-2">
                <div>
                  <p className="text-xs text-gray-500">Disponible</p>
                  <p className="text-lg font-bold text-gray-900">
                    {formatCurrency(balance.available_balance, balance.currency)}
                  </p>
                </div>
                {balance.reserved_balance > 0 && (
                  <div>
                    <p className="text-xs text-gray-500">Réservé</p>
                    <p className="text-sm font-medium text-yellow-600">
                      {formatCurrency(balance.reserved_balance, balance.currency)}
                    </p>
                  </div>
                )}
              </div>
            </div>
          );
        })}
      </div>

      {/* Transactions History */}
      <div className="bg-white rounded-xl shadow-sm">
        <div className="p-6 border-b border-gray-200">
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-semibold text-gray-900">
              Historique des Transactions
            </h2>
            <div className="flex items-center space-x-4">
              <select
                value={selectedCurrency}
                onChange={(e) => setSelectedCurrency(e.target.value)}
                className="px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-orange-500 focus:border-transparent"
              >
                <option value="all">Toutes devises</option>
                {CURRENCIES.map(currency => (
                  <option key={currency.code} value={currency.code}>
                    {currency.flag} {currency.code}
                  </option>
                ))}
              </select>
              <button
                onClick={() => refetchBalances()}
                className="p-2 text-gray-600 hover:text-gray-900"
              >
                <RefreshCw className="h-5 w-5" />
              </button>
            </div>
          </div>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full">
            <thead className="bg-gray-50">
              <tr>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                  Type
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                  Montant
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                  Devise
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                  Description
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                  Date
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-200">
              {transactions?.items?.length === 0 ? (
                <tr>
                  <td colSpan={5} className="px-6 py-12 text-center text-gray-500">
                    Aucune transaction trouvée
                  </td>
                </tr>
              ) : (
                transactions?.items?.map((transaction) => (
                  <tr key={transaction.id} className="hover:bg-gray-50">
                    <td className="px-6 py-4 whitespace-nowrap">
                      <div className="flex items-center">
                        <div className={`p-2 rounded-lg ${
                          transaction.type === 'deposit' ? 'bg-green-100' :
                          transaction.type === 'withdrawal' ? 'bg-red-100' :
                          transaction.type === 'transfer_in' ? 'bg-blue-100' :
                          transaction.type === 'transfer_out' ? 'bg-purple-100' :
                          'bg-yellow-100'
                        }`}>
                          {transaction.type === 'deposit' ? (
                            <ArrowDownRight className="h-4 w-4 text-green-600" />
                          ) : transaction.type === 'withdrawal' ? (
                            <ArrowUpRight className="h-4 w-4 text-red-600" />
                          ) : transaction.type === 'transfer_in' ? (
                            <Download className="h-4 w-4 text-blue-600" />
                          ) : transaction.type === 'transfer_out' ? (
                            <Send className="h-4 w-4 text-purple-600" />
                          ) : (
                            <Shield className="h-4 w-4 text-yellow-600" />
                          )}
                        </div>
                        <span className="ml-3 text-sm font-medium text-gray-900">
                          {transaction.type === 'deposit' ? 'Dépôt' :
                           transaction.type === 'withdrawal' ? 'Retrait' :
                           transaction.type === 'transfer_in' ? 'Transfert reçu' :
                           transaction.type === 'transfer_out' ? 'Transfert envoyé' :
                           transaction.type === 'reserve' ? 'Réservation' :
                           'Libération'}
                        </span>
                      </div>
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap">
                      <span className={`font-semibold ${
                        transaction.type === 'deposit' || transaction.type === 'transfer_in' || transaction.type === 'release'
                          ? 'text-green-600' : 'text-red-600'
                      }`}>
                        {transaction.type === 'deposit' || transaction.type === 'transfer_in' || transaction.type === 'release' ? '+' : '-'}
                        {formatCurrency(transaction.amount, transaction.currency)}
                      </span>
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap">
                      <span className="text-sm text-gray-900">
                        {transaction.currency}
                      </span>
                    </td>
                    <td className="px-6 py-4">
                      <span className="text-sm text-gray-600">
                        {transaction.description || '-'}
                      </span>
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap">
                      <span className="text-sm text-gray-500">
                        {formatDateTime(transaction.created_at)}
                      </span>
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </div>

      {/* Info Card */}
      <div className="mt-8 bg-blue-50 border border-blue-200 rounded-xl p-6">
        <div className="flex items-start">
          <Info className="h-5 w-5 text-blue-600 mt-0.5 mr-3 flex-shrink-0" />
          <div>
            <h3 className="text-sm font-semibold text-blue-900 mb-1">
              Sécurité de vos fonds
            </h3>
            <p className="text-sm text-blue-700">
              Vos fonds sont sécurisés et protégés. Les transactions sont vérifiées
              et validées avant d'être exécutées. Pour toute question, contactez notre
              support disponible 24/7.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
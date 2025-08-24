'use client';

import { useState } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useQuery, useMutation } from '@tanstack/react-query';
import { ArrowDownUp, Loader2, TrendingUp, TrendingDown, Clock, AlertCircle } from 'lucide-react';
import { InitiateExchangeInput, initiateExchangeSchema } from '@/lib/utils/validators';
import { formatCurrency, formatRelativeTime } from '@/lib/utils/formatters';
import { CURRENCIES, PAYMENT_METHODS, URGENCY_LEVELS } from '@/lib/utils/constants';
import { exchangeApi } from '@/lib/api/exchange';
import { ratesApi } from '@/lib/api/rates';
import { useRouter } from 'next/navigation';

export default function ExchangePage() {
  const router = useRouter();
  const [estimatedAmount, setEstimatedAmount] = useState<number | null>(null);
  const [selectedRate, setSelectedRate] = useState<any>(null);
  const [isSwapped, setIsSwapped] = useState(false);

  const {
    register,
    handleSubmit,
    formState: { errors },
    watch,
    setValue,
  } = useForm<InitiateExchangeInput>({
    resolver: zodResolver(initiateExchangeSchema),
    defaultValues: {
      from_currency: 'EUR',
      to_currency: 'XOF',
      amount: 0,
      operation: 'sell',
      payment_method: 'cash',
      urgency_level: 'Normal',
    },
  });

  const fromCurrency = watch('from_currency');
  const toCurrency = watch('to_currency');
  const amount = watch('amount');

  // Swap currencies
  const handleSwap = () => {
    const temp = fromCurrency;
    setValue('from_currency', toCurrency);
    setValue('to_currency', temp);
    setIsSwapped(!isSwapped);
  };

  // Get best rates
  const { data: bestRates } = useQuery({
    queryKey: ['bestRates', fromCurrency, toCurrency],
    queryFn: () => ratesApi.getBestRate(fromCurrency, toCurrency),
    refetchInterval: 30000,
  });

  // Get quote when amount changes
  const { data: quote, isLoading: isQuoteLoading } = useQuery({
    queryKey: ['quote', fromCurrency, toCurrency, amount],
    queryFn: async () => {
      if (!amount || amount <= 0) return null;
      const response = await ratesApi.getQuote(fromCurrency, toCurrency, amount);
      setEstimatedAmount(response.amount_to);
      setSelectedRate(response);
      return response;
    },
    enabled: !!amount && amount > 0,
  });

  // Create transaction mutation
  const createTransactionMutation = useMutation({
    mutationFn: exchangeApi.initiateTransaction,
    onSuccess: (data) => {
      router.push(`/exchange/${data.id}`);
    },
    onError: (error: any) => {
      console.error('Transaction error:', error);
    },
  });

  const onSubmit = (data: InitiateExchangeInput) => {
    createTransactionMutation.mutate(data);
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-orange-50 to-green-50 py-8 px-4 sm:px-6 lg:px-8">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="text-center mb-8">
          <h1 className="text-3xl font-bold text-gray-900">Échanger des devises</h1>
          <p className="mt-2 text-gray-600">
            Obtenez les meilleurs taux de change en temps réel
          </p>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
          {/* Main Exchange Form */}
          <div className="lg:col-span-2">
            <form onSubmit={handleSubmit(onSubmit)} className="bg-white rounded-xl shadow-lg p-6 space-y-6">
              {/* Currency Selection */}
              <div className="space-y-4">
                <div className="grid grid-cols-2 gap-4 items-center">
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-2">
                      De
                    </label>
                    <select
                      {...register('from_currency')}
                      className="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-orange-500 focus:border-transparent"
                    >
                      {CURRENCIES.map(currency => (
                        <option key={currency.code} value={currency.code}>
                          {currency.flag} {currency.code} - {currency.name}
                        </option>
                      ))}
                    </select>
                  </div>

                  <div className="flex justify-center">
                    <button
                      type="button"
                      onClick={handleSwap}
                      className="p-3 bg-gray-100 rounded-full hover:bg-gray-200 transition-colors"
                    >
                      <ArrowDownUp className="h-5 w-5 text-gray-600" />
                    </button>
                  </div>

                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-2">
                      Vers
                    </label>
                    <select
                      {...register('to_currency')}
                      className="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-orange-500 focus:border-transparent"
                    >
                      {CURRENCIES.map(currency => (
                        <option key={currency.code} value={currency.code}>
                          {currency.flag} {currency.code} - {currency.name}
                        </option>
                      ))}
                    </select>
                  </div>
                </div>

                {/* Amount Input */}
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-2">
                    Montant
                  </label>
                  <div className="relative">
                    <input
                      {...register('amount', { valueAsNumber: true })}
                      type="number"
                      step="0.01"
                      placeholder="0.00"
                      className="w-full px-4 py-4 text-2xl font-bold border border-gray-300 rounded-lg focus:ring-2 focus:ring-orange-500 focus:border-transparent"
                    />
                    <span className="absolute right-4 top-1/2 transform -translate-y-1/2 text-gray-500">
                      {fromCurrency}
                    </span>
                  </div>
                  {errors.amount && (
                    <p className="mt-1 text-sm text-red-600">{errors.amount.message}</p>
                  )}
                </div>

                {/* Estimated Amount */}
                {estimatedAmount && (
                  <div className="bg-green-50 border border-green-200 rounded-lg p-4">
                    <p className="text-sm text-gray-600">Vous recevrez environ</p>
                    <p className="text-3xl font-bold text-green-600">
                      {formatCurrency(estimatedAmount, toCurrency)}
                    </p>
                    {quote && (
                      <div className="mt-2 space-y-1 text-sm text-gray-600">
                        <p>Taux: 1 {fromCurrency} = {quote.rate} {toCurrency}</p>
                        <p>Frais: {formatCurrency(quote.fee, fromCurrency)}</p>
                      </div>
                    )}
                  </div>
                )}
              </div>

              {/* Additional Options */}
              <div className="space-y-4">
                {/* Payment Method */}
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-2">
                    Mode de paiement
                  </label>
                  <select
                    {...register('payment_method')}
                    className="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-orange-500 focus:border-transparent"
                  >
                    {PAYMENT_METHODS.map(method => (
                      <option key={method.value} value={method.value}>
                        {method.icon} {method.label}
                      </option>
                    ))}
                  </select>
                </div>

                {/* Urgency Level */}
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-2">
                    Niveau d'urgence
                  </label>
                  <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
                    {URGENCY_LEVELS.map(level => (
                      <label
                        key={level.value}
                        className={`relative flex cursor-pointer rounded-lg border p-3 focus:outline-none ${
                          watch('urgency_level') === level.value
                            ? 'border-orange-500 bg-orange-50'
                            : 'border-gray-200'
                        }`}
                      >
                        <input
                          {...register('urgency_level')}
                          type="radio"
                          value={level.value}
                          className="sr-only"
                        />
                        <div className="flex flex-col items-center w-full">
                          <span className={`text-sm font-medium text-${level.color}-600`}>
                            {level.value}
                          </span>
                          <span className="text-xs text-gray-500">{level.label}</span>
                        </div>
                      </label>
                    ))}
                  </div>
                </div>

                {/* Notes */}
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-2">
                    Notes (optionnel)
                  </label>
                  <textarea
                    {...register('notes')}
                    rows={3}
                    className="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-orange-500 focus:border-transparent"
                    placeholder="Instructions spéciales..."
                  />
                </div>
              </div>

              {/* Submit Button */}
              <button
                type="submit"
                disabled={createTransactionMutation.isPending || !amount || amount <= 0}
                className="w-full py-4 px-6 bg-gradient-to-r from-orange-500 to-orange-600 text-white font-medium rounded-lg hover:from-orange-600 hover:to-orange-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-orange-500 disabled:opacity-50 disabled:cursor-not-allowed transition-all flex items-center justify-center"
              >
                {createTransactionMutation.isPending ? (
                  <>
                    <Loader2 className="animate-spin -ml-1 mr-3 h-5 w-5" />
                    Traitement en cours...
                  </>
                ) : (
                  <>
                    <ArrowDownUp className="mr-2 h-5 w-5" />
                    Échanger maintenant
                  </>
                )}
              </button>
            </form>
          </div>

          {/* Sidebar - Market Info */}
          <div className="space-y-6">
            {/* Live Rates */}
            <div className="bg-white rounded-xl shadow-lg p-6">
              <h3 className="text-lg font-semibold text-gray-900 mb-4">
                Taux en temps réel
              </h3>
              <div className="space-y-3">
                <div className="flex justify-between items-center">
                  <span className="text-gray-600">EUR/XOF</span>
                  <div className="text-right">
                    <p className="font-semibold">655.96</p>
                    <p className="text-xs text-green-600 flex items-center justify-end">
                      <TrendingUp className="h-3 w-3 mr-1" />
                      +0.12%
                    </p>
                  </div>
                </div>
                <div className="flex justify-between items-center">
                  <span className="text-gray-600">USD/XOF</span>
                  <div className="text-right">
                    <p className="font-semibold">598.45</p>
                    <p className="text-xs text-red-600 flex items-center justify-end">
                      <TrendingDown className="h-3 w-3 mr-1" />
                      -0.08%
                    </p>
                  </div>
                </div>
              </div>
            </div>

            {/* Information */}
            <div className="bg-blue-50 border border-blue-200 rounded-xl p-6">
              <div className="flex items-start">
                <AlertCircle className="h-5 w-5 text-blue-600 mt-0.5 mr-3 flex-shrink-0" />
                <div>
                  <h4 className="text-sm font-semibold text-blue-900 mb-1">
                    Information importante
                  </h4>
                  <p className="text-xs text-blue-700">
                    Les transactions sont valides pendant 15 minutes après initiation.
                    Assurez-vous d'avoir les fonds disponibles avant de confirmer.
                  </p>
                </div>
              </div>
            </div>

            {/* Support */}
            <div className="bg-white rounded-xl shadow-lg p-6">
              <h3 className="text-lg font-semibold text-gray-900 mb-4">
                Besoin d'aide ?
              </h3>
              <div className="space-y-3">
                <a href="#" className="block text-orange-600 hover:text-orange-700 text-sm">
                  Guide d'utilisation →
                </a>
                <a href="#" className="block text-orange-600 hover:text-orange-700 text-sm">
                  FAQ →
                </a>
                <a href="#" className="block text-orange-600 hover:text-orange-700 text-sm">
                  Contacter le support →
                </a>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
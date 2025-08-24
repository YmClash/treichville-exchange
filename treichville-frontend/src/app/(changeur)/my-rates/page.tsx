'use client';

import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { 
  Plus, Edit2, Trash2, TrendingUp, TrendingDown, 
  DollarSign, RefreshCw, Save, X, AlertCircle, Loader2
} from 'lucide-react';
import { ratesApi } from '@/lib/api/rates';
import { CreateRateInput, createRateSchema } from '@/lib/utils/validators';
import { formatCurrency, formatDateTime } from '@/lib/utils/formatters';
import { CURRENCIES } from '@/lib/utils/constants';
import { toast } from '@/lib/hooks/useToast';

export default function MyRatesPage() {
  const queryClient = useQueryClient();
  const [isAddModalOpen, setIsAddModalOpen] = useState(false);
  const [editingRate, setEditingRate] = useState<any>(null);

  // Fetch changeur's rates
  const { data: myRates, isLoading } = useQuery({
    queryKey: ['changeurRates'],
    queryFn: ratesApi.getChangeurRates,
    refetchInterval: 30000,
  });

  // Create rate mutation
  const createRateMutation = useMutation({
    mutationFn: ratesApi.createRate,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['changeurRates'] });
      setIsAddModalOpen(false);
      toast({
        title: 'Taux créé avec succès',
        variant: 'success',
      });
    },
    onError: (error: any) => {
      toast({
        title: 'Erreur',
        description: error.message || 'Impossible de créer le taux',
        variant: 'destructive',
      });
    },
  });

  // Update rate mutation
  const updateRateMutation = useMutation({
    mutationFn: ({ id, data }: { id: string; data: any }) => 
      ratesApi.updateRate(id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['changeurRates'] });
      setEditingRate(null);
      toast({
        title: 'Taux mis à jour avec succès',
        variant: 'success',
      });
    },
  });

  // Delete rate mutation
  const deleteRateMutation = useMutation({
    mutationFn: ratesApi.deleteRate,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['changeurRates'] });
      toast({
        title: 'Taux supprimé avec succès',
        variant: 'success',
      });
    },
  });

  // Form for creating/editing rates
  const {
    register,
    handleSubmit,
    formState: { errors },
    reset,
    setValue,
  } = useForm<CreateRateInput>({
    resolver: zodResolver(createRateSchema),
    defaultValues: {
      from_currency: 'EUR',
      to_currency: 'XOF',
      rate_type: 'sell',
      rate: 0,
      min_amount: 0,
      max_amount: 0,
    },
  });

  const onSubmit = (data: CreateRateInput) => {
    if (editingRate) {
      updateRateMutation.mutate({ id: editingRate.id, data });
    } else {
      createRateMutation.mutate(data);
    }
  };

  const handleEdit = (rate: any) => {
    setEditingRate(rate);
    setValue('from_currency', rate.from_currency);
    setValue('to_currency', rate.to_currency);
    setValue('rate_type', rate.rate_type);
    setValue('rate', rate.rate);
    setValue('min_amount', rate.min_amount);
    setValue('max_amount', rate.max_amount);
    setValue('available_amount', rate.available_amount);
    setIsAddModalOpen(true);
  };

  const handleToggleActive = (rate: any) => {
    updateRateMutation.mutate({
      id: rate.id,
      data: { is_active: !rate.is_active },
    });
  };

  // Group rates by currency pair
  const groupedRates = myRates?.reduce((acc: any, rate: any) => {
    const key = `${rate.from_currency}-${rate.to_currency}`;
    if (!acc[key]) {
      acc[key] = [];
    }
    acc[key].push(rate);
    return acc;
  }, {});

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      {/* Header */}
      <div className="mb-8 flex justify-between items-center">
        <div>
          <h1 className="text-3xl font-bold text-gray-900">Mes Taux de Change</h1>
          <p className="text-gray-600 mt-1">
            Gérez vos taux et restez compétitif
          </p>
        </div>
        <button
          onClick={() => {
            setEditingRate(null);
            reset();
            setIsAddModalOpen(true);
          }}
          className="flex items-center px-4 py-2 bg-gradient-to-r from-orange-500 to-orange-600 text-white rounded-lg hover:from-orange-600 hover:to-orange-700 transition-all"
        >
          <Plus className="h-5 w-5 mr-2" />
          Ajouter un taux
        </button>
      </div>

      {/* Statistics Cards */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-6 mb-8">
        <div className="bg-white rounded-xl shadow-sm p-6">
          <div className="flex items-center justify-between mb-2">
            <DollarSign className="h-8 w-8 text-orange-500" />
            <span className="text-xs text-gray-500">Total</span>
          </div>
          <p className="text-2xl font-bold text-gray-900">
            {myRates?.length || 0}
          </p>
          <p className="text-sm text-gray-600">Taux actifs</p>
        </div>

        <div className="bg-white rounded-xl shadow-sm p-6">
          <div className="flex items-center justify-between mb-2">
            <TrendingUp className="h-8 w-8 text-green-500" />
            <span className="text-xs text-gray-500">Achat</span>
          </div>
          <p className="text-2xl font-bold text-gray-900">
            {myRates?.filter(r => r.rate_type === 'buy').length || 0}
          </p>
          <p className="text-sm text-gray-600">Taux d'achat</p>
        </div>

        <div className="bg-white rounded-xl shadow-sm p-6">
          <div className="flex items-center justify-between mb-2">
            <TrendingDown className="h-8 w-8 text-blue-500" />
            <span className="text-xs text-gray-500">Vente</span>
          </div>
          <p className="text-2xl font-bold text-gray-900">
            {myRates?.filter(r => r.rate_type === 'sell').length || 0}
          </p>
          <p className="text-sm text-gray-600">Taux de vente</p>
        </div>

        <div className="bg-white rounded-xl shadow-sm p-6">
          <div className="flex items-center justify-between mb-2">
            <RefreshCw className="h-8 w-8 text-purple-500" />
            <span className="text-xs text-gray-500">Màj</span>
          </div>
          <p className="text-2xl font-bold text-gray-900">
            {new Date().getHours()}:{String(new Date().getMinutes()).padStart(2, '0')}
          </p>
          <p className="text-sm text-gray-600">Dernière mise à jour</p>
        </div>
      </div>

      {/* Rates List */}
      {isLoading ? (
        <div className="flex items-center justify-center py-12">
          <Loader2 className="h-8 w-8 animate-spin text-orange-500" />
        </div>
      ) : !groupedRates || Object.keys(groupedRates).length === 0 ? (
        <div className="bg-white rounded-xl shadow-sm p-12 text-center">
          <AlertCircle className="h-12 w-12 text-gray-400 mx-auto mb-4" />
          <h3 className="text-lg font-medium text-gray-900 mb-2">
            Aucun taux configuré
          </h3>
          <p className="text-gray-600 mb-6">
            Commencez par ajouter vos premiers taux de change
          </p>
          <button
            onClick={() => setIsAddModalOpen(true)}
            className="inline-flex items-center px-4 py-2 bg-gradient-to-r from-orange-500 to-orange-600 text-white rounded-lg hover:from-orange-600 hover:to-orange-700"
          >
            <Plus className="h-5 w-5 mr-2" />
            Ajouter un taux
          </button>
        </div>
      ) : (
        <div className="space-y-6">
          {Object.entries(groupedRates).map(([pair, rates]: [string, any]) => (
            <div key={pair} className="bg-white rounded-xl shadow-sm overflow-hidden">
              <div className="px-6 py-4 bg-gray-50 border-b">
                <h3 className="text-lg font-semibold text-gray-900">
                  {pair.replace('-', ' → ')}
                </h3>
              </div>
              <div className="divide-y divide-gray-200">
                {rates.map((rate: any) => (
                  <div key={rate.id} className="px-6 py-4">
                    <div className="flex items-center justify-between">
                      <div className="flex-1">
                        <div className="flex items-center space-x-4">
                          <span className={`px-3 py-1 rounded-full text-sm font-medium ${
                            rate.rate_type === 'buy' 
                              ? 'bg-green-100 text-green-700'
                              : 'bg-blue-100 text-blue-700'
                          }`}>
                            {rate.rate_type === 'buy' ? 'Achat' : 'Vente'}
                          </span>
                          <span className="text-2xl font-bold text-gray-900">
                            {rate.rate}
                          </span>
                          <span className={`px-2 py-1 rounded-full text-xs ${
                            rate.is_active
                              ? 'bg-green-100 text-green-700'
                              : 'bg-gray-100 text-gray-700'
                          }`}>
                            {rate.is_active ? 'Actif' : 'Inactif'}
                          </span>
                        </div>
                        <div className="mt-2 grid grid-cols-3 gap-4 text-sm">
                          <div>
                            <span className="text-gray-500">Min:</span>{' '}
                            <span className="font-medium">
                              {formatCurrency(rate.min_amount, rate.from_currency)}
                            </span>
                          </div>
                          <div>
                            <span className="text-gray-500">Max:</span>{' '}
                            <span className="font-medium">
                              {formatCurrency(rate.max_amount, rate.from_currency)}
                            </span>
                          </div>
                          {rate.available_amount && (
                            <div>
                              <span className="text-gray-500">Disponible:</span>{' '}
                              <span className="font-medium">
                                {formatCurrency(rate.available_amount, rate.to_currency)}
                              </span>
                            </div>
                          )}
                        </div>
                      </div>
                      <div className="flex items-center space-x-2">
                        <button
                          onClick={() => handleToggleActive(rate)}
                          className={`p-2 rounded-lg ${
                            rate.is_active
                              ? 'text-gray-600 hover:bg-gray-100'
                              : 'text-green-600 hover:bg-green-50'
                          }`}
                        >
                          {rate.is_active ? (
                            <X className="h-5 w-5" />
                          ) : (
                            <RefreshCw className="h-5 w-5" />
                          )}
                        </button>
                        <button
                          onClick={() => handleEdit(rate)}
                          className="p-2 text-blue-600 hover:bg-blue-50 rounded-lg"
                        >
                          <Edit2 className="h-5 w-5" />
                        </button>
                        <button
                          onClick={() => {
                            if (confirm('Êtes-vous sûr de vouloir supprimer ce taux ?')) {
                              deleteRateMutation.mutate(rate.id);
                            }
                          }}
                          className="p-2 text-red-600 hover:bg-red-50 rounded-lg"
                        >
                          <Trash2 className="h-5 w-5" />
                        </button>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Add/Edit Modal */}
      {isAddModalOpen && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4">
          <div className="bg-white rounded-xl shadow-xl max-w-md w-full p-6">
            <h2 className="text-xl font-semibold text-gray-900 mb-6">
              {editingRate ? 'Modifier le taux' : 'Ajouter un taux'}
            </h2>
            
            <form onSubmit={handleSubmit(onSubmit)} className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    De
                  </label>
                  <select
                    {...register('from_currency')}
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-orange-500"
                  >
                    {CURRENCIES.map(currency => (
                      <option key={currency.code} value={currency.code}>
                        {currency.flag} {currency.code}
                      </option>
                    ))}
                  </select>
                </div>
                
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    Vers
                  </label>
                  <select
                    {...register('to_currency')}
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-orange-500"
                  >
                    {CURRENCIES.map(currency => (
                      <option key={currency.code} value={currency.code}>
                        {currency.flag} {currency.code}
                      </option>
                    ))}
                  </select>
                </div>
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  Type
                </label>
                <select
                  {...register('rate_type')}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-orange-500"
                >
                  <option value="buy">Achat</option>
                  <option value="sell">Vente</option>
                </select>
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  Taux
                </label>
                <input
                  {...register('rate', { valueAsNumber: true })}
                  type="number"
                  step="0.0001"
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-orange-500"
                />
                {errors.rate && (
                  <p className="text-red-500 text-sm mt-1">{errors.rate.message}</p>
                )}
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    Montant Min
                  </label>
                  <input
                    {...register('min_amount', { valueAsNumber: true })}
                    type="number"
                    step="0.01"
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-orange-500"
                  />
                </div>
                
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    Montant Max
                  </label>
                  <input
                    {...register('max_amount', { valueAsNumber: true })}
                    type="number"
                    step="0.01"
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-orange-500"
                  />
                </div>
              </div>

              <div className="flex justify-end space-x-3 pt-4">
                <button
                  type="button"
                  onClick={() => {
                    setIsAddModalOpen(false);
                    setEditingRate(null);
                    reset();
                  }}
                  className="px-4 py-2 text-gray-700 bg-gray-100 rounded-lg hover:bg-gray-200"
                >
                  Annuler
                </button>
                <button
                  type="submit"
                  disabled={createRateMutation.isPending || updateRateMutation.isPending}
                  className="flex items-center px-4 py-2 bg-gradient-to-r from-orange-500 to-orange-600 text-white rounded-lg hover:from-orange-600 hover:to-orange-700 disabled:opacity-50"
                >
                  {(createRateMutation.isPending || updateRateMutation.isPending) ? (
                    <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                  ) : (
                    <Save className="h-4 w-4 mr-2" />
                  )}
                  {editingRate ? 'Modifier' : 'Ajouter'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
}
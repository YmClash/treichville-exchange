'use client';

import { useEffect, useState } from 'react';
import { useParams, useRouter } from 'next/navigation';
import { useQuery, useMutation } from '@tanstack/react-query';
import { 
  ArrowLeft, Clock, CheckCircle, XCircle, AlertCircle,
  Copy, Share2, Download, CreditCard, MapPin, Phone,
  User, Calendar, TrendingUp, Loader2, RefreshCw
} from 'lucide-react';
import { exchangeApi } from '@/lib/api/exchange';
import { formatCurrency, formatDateTime, formatRelativeTime, formatPhone } from '@/lib/utils/formatters';
import { TRANSACTION_STATUS, PAYMENT_METHODS } from '@/lib/utils/constants';
import { toast } from '@/lib/hooks/useToast';

export default function TransactionDetailsPage() {
  const params = useParams();
  const router = useRouter();
  const transactionId = params.id as string;
  const [isUploading, setIsUploading] = useState(false);
  const [timeRemaining, setTimeRemaining] = useState<string>('');

  // Fetch transaction details
  const { data: transaction, isLoading, refetch } = useQuery({
    queryKey: ['transaction', transactionId],
    queryFn: () => exchangeApi.getTransaction(transactionId),
    refetchInterval: 10000, // Refresh every 10 seconds
  });

  // Complete transaction mutation
  const completeMutation = useMutation({
    mutationFn: () => exchangeApi.completeTransaction(transactionId),
    onSuccess: () => {
      refetch();
      toast({
        title: 'Transaction complétée avec succès',
        variant: 'success',
      });
    },
  });

  // Cancel transaction mutation
  const cancelMutation = useMutation({
    mutationFn: (reason: string) => exchangeApi.cancelTransaction(transactionId, reason),
    onSuccess: () => {
      refetch();
      toast({
        title: 'Transaction annulée',
        variant: 'success',
      });
      router.push('/transactions');
    },
  });

  // Confirm payment mutation
  const confirmPaymentMutation = useMutation({
    mutationFn: (data: { payment_reference: string }) => 
      exchangeApi.confirmPayment({
        transaction_id: transactionId,
        payment_reference: data.payment_reference,
      }),
    onSuccess: () => {
      refetch();
      toast({
        title: 'Paiement confirmé',
        description: 'Le changeur a été notifié',
        variant: 'success',
      });
    },
  });

  // Calculate time remaining
  useEffect(() => {
    if (transaction?.expires_at && transaction.status === 'pending') {
      const interval = setInterval(() => {
        const expiresAt = new Date(transaction.expires_at!);
        const now = new Date();
        const diff = expiresAt.getTime() - now.getTime();

        if (diff <= 0) {
          setTimeRemaining('Expiré');
          clearInterval(interval);
        } else {
          const minutes = Math.floor(diff / 60000);
          const seconds = Math.floor((diff % 60000) / 1000);
          setTimeRemaining(`${minutes}:${seconds.toString().padStart(2, '0')}`);
        }
      }, 1000);

      return () => clearInterval(interval);
    }
  }, [transaction]);

  const getStatusIcon = (status: string) => {
    switch(status) {
      case 'completed':
        return <CheckCircle className="h-6 w-6 text-green-600" />;
      case 'cancelled':
      case 'failed':
      case 'expired':
        return <XCircle className="h-6 w-6 text-red-600" />;
      case 'pending':
        return <Clock className="h-6 w-6 text-yellow-600" />;
      case 'paid':
      case 'confirmed':
        return <AlertCircle className="h-6 w-6 text-blue-600" />;
      default:
        return <AlertCircle className="h-6 w-6 text-gray-600" />;
    }
  };

  const getStatusColor = (status: string) => {
    switch(status) {
      case 'completed': return 'bg-green-100 text-green-700';
      case 'cancelled':
      case 'failed':
      case 'expired': return 'bg-red-100 text-red-700';
      case 'pending': return 'bg-yellow-100 text-yellow-700';
      case 'paid':
      case 'confirmed': return 'bg-blue-100 text-blue-700';
      default: return 'bg-gray-100 text-gray-700';
    }
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    toast({
      title: 'Copié dans le presse-papier',
    });
  };

  if (isLoading) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <Loader2 className="h-8 w-8 animate-spin text-orange-500" />
      </div>
    );
  }

  if (!transaction) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="text-center">
          <XCircle className="h-12 w-12 text-red-500 mx-auto mb-4" />
          <h2 className="text-xl font-semibold text-gray-900">Transaction introuvable</h2>
          <button
            onClick={() => router.push('/transactions')}
            className="mt-4 text-orange-600 hover:text-orange-700"
          >
            Retour aux transactions
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      {/* Header */}
      <div className="mb-8">
        <button
          onClick={() => router.push('/transactions')}
          className="flex items-center text-gray-600 hover:text-gray-900 mb-4"
        >
          <ArrowLeft className="h-5 w-5 mr-2" />
          Retour aux transactions
        </button>
        
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-3xl font-bold text-gray-900">
              Transaction #{transaction.reference}
            </h1>
            <p className="text-gray-600 mt-1">
              Créée {formatRelativeTime(transaction.created_at)}
            </p>
          </div>
          <div className="flex items-center space-x-3">
            <button
              onClick={() => refetch()}
              className="p-2 text-gray-600 hover:text-gray-900"
            >
              <RefreshCw className="h-5 w-5" />
            </button>
            <button
              onClick={() => copyToClipboard(transaction.reference)}
              className="p-2 text-gray-600 hover:text-gray-900"
            >
              <Copy className="h-5 w-5" />
            </button>
            <button className="p-2 text-gray-600 hover:text-gray-900">
              <Share2 className="h-5 w-5" />
            </button>
          </div>
        </div>
      </div>

      {/* Status Alert */}
      <div className={`rounded-xl p-6 mb-8 ${
        transaction.status === 'completed' ? 'bg-green-50 border border-green-200' :
        transaction.status === 'pending' ? 'bg-yellow-50 border border-yellow-200' :
        transaction.status === 'cancelled' || transaction.status === 'failed' ? 'bg-red-50 border border-red-200' :
        'bg-blue-50 border border-blue-200'
      }`}>
        <div className="flex items-start">
          {getStatusIcon(transaction.status)}
          <div className="ml-4 flex-1">
            <h3 className="text-lg font-semibold text-gray-900">
              {TRANSACTION_STATUS[transaction.status]?.label || transaction.status}
            </h3>
            <p className="text-gray-700 mt-1">
              {transaction.status === 'pending' && 'En attente du paiement du client'}
              {transaction.status === 'paid' && 'Paiement reçu, en attente de confirmation'}
              {transaction.status === 'confirmed' && 'Transaction confirmée par le changeur'}
              {transaction.status === 'completed' && 'Transaction complétée avec succès'}
              {transaction.status === 'cancelled' && `Annulée: ${transaction.cancelled_reason || 'Par l\'utilisateur'}`}
              {transaction.status === 'expired' && 'Transaction expirée'}
              {transaction.status === 'failed' && 'Transaction échouée'}
            </p>
            
            {transaction.status === 'pending' && transaction.expires_at && (
              <div className="mt-3 flex items-center space-x-4">
                <div className="flex items-center text-sm">
                  <Clock className="h-4 w-4 mr-1" />
                  <span className="font-medium">Temps restant: {timeRemaining}</span>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
        {/* Main Details */}
        <div className="lg:col-span-2 space-y-6">
          {/* Exchange Details */}
          <div className="bg-white rounded-xl shadow-sm p-6">
            <h3 className="text-lg font-semibold text-gray-900 mb-4">
              Détails de l'échange
            </h3>
            
            <div className="space-y-4">
              <div className="flex items-center justify-between p-4 bg-gray-50 rounded-lg">
                <div>
                  <p className="text-sm text-gray-500">Vous envoyez</p>
                  <p className="text-2xl font-bold text-gray-900">
                    {formatCurrency(transaction.amount_from, transaction.from_currency)}
                  </p>
                </div>
                <TrendingUp className="h-8 w-8 text-orange-500" />
                <div className="text-right">
                  <p className="text-sm text-gray-500">Vous recevez</p>
                  <p className="text-2xl font-bold text-green-600">
                    {formatCurrency(transaction.amount_to, transaction.to_currency)}
                  </p>
                </div>
              </div>

              <div className="grid grid-cols-2 gap-4 text-sm">
                <div>
                  <p className="text-gray-500">Taux appliqué</p>
                  <p className="font-medium">
                    1 {transaction.from_currency} = {transaction.rate_applied} {transaction.to_currency}
                  </p>
                </div>
                <div>
                  <p className="text-gray-500">Frais</p>
                  <p className="font-medium">
                    {formatCurrency(transaction.fee, transaction.from_currency)}
                  </p>
                </div>
                <div>
                  <p className="text-gray-500">Total à payer</p>
                  <p className="font-medium">
                    {formatCurrency(transaction.total_amount, transaction.from_currency)}
                  </p>
                </div>
                <div>
                  <p className="text-gray-500">Méthode de paiement</p>
                  <p className="font-medium">
                    {PAYMENT_METHODS.find(m => m.value === transaction.payment_method)?.label}
                  </p>
                </div>
              </div>
            </div>
          </div>

          {/* Payment Instructions (if pending) */}
          {transaction.status === 'pending' && (
            <div className="bg-white rounded-xl shadow-sm p-6">
              <h3 className="text-lg font-semibold text-gray-900 mb-4">
                Instructions de paiement
              </h3>
              
              <div className="space-y-4">
                {transaction.payment_method === 'cash' ? (
                  <>
                    <div className="flex items-start space-x-3">
                      <MapPin className="h-5 w-5 text-gray-400 mt-0.5" />
                      <div>
                        <p className="font-medium text-gray-900">Point de rencontre</p>
                        <p className="text-gray-600">Rue 12, Treichville - Stand 47</p>
                      </div>
                    </div>
                    <div className="flex items-start space-x-3">
                      <User className="h-5 w-5 text-gray-400 mt-0.5" />
                      <div>
                        <p className="font-medium text-gray-900">Changeur</p>
                        <p className="text-gray-600">Mohamed Koné</p>
                      </div>
                    </div>
                    <div className="flex items-start space-x-3">
                      <Phone className="h-5 w-5 text-gray-400 mt-0.5" />
                      <div>
                        <p className="font-medium text-gray-900">Contact</p>
                        <p className="text-gray-600">{formatPhone('0712345678')}</p>
                      </div>
                    </div>
                  </>
                ) : (
                  <>
                    <div className="p-4 bg-orange-50 border border-orange-200 rounded-lg">
                      <p className="font-medium text-orange-900 mb-2">
                        Envoyez {formatCurrency(transaction.total_amount, transaction.from_currency)} via {
                          PAYMENT_METHODS.find(m => m.value === transaction.payment_method)?.label
                        }
                      </p>
                      <p className="text-sm text-orange-700">
                        Numéro: <span className="font-mono font-bold">07 12 34 56 78</span>
                      </p>
                      <p className="text-sm text-orange-700 mt-1">
                        Référence: <span className="font-mono font-bold">{transaction.reference}</span>
                      </p>
                    </div>
                    
                    <div>
                      <label className="block text-sm font-medium text-gray-700 mb-2">
                        Référence de paiement
                      </label>
                      <div className="flex space-x-2">
                        <input
                          type="text"
                          placeholder="Entrez la référence de votre paiement"
                          className="flex-1 px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-orange-500"
                        />
                        <button
                          onClick={() => confirmPaymentMutation.mutate({ payment_reference: 'REF123' })}
                          className="px-4 py-2 bg-gradient-to-r from-orange-500 to-orange-600 text-white rounded-lg hover:from-orange-600 hover:to-orange-700"
                        >
                          Confirmer
                        </button>
                      </div>
                    </div>
                  </>
                )}
              </div>
            </div>
          )}

          {/* Timeline */}
          <div className="bg-white rounded-xl shadow-sm p-6">
            <h3 className="text-lg font-semibold text-gray-900 mb-4">
              Historique
            </h3>
            
            <div className="space-y-4">
              {transaction.created_at && (
                <div className="flex items-start space-x-3">
                  <div className="w-2 h-2 bg-gray-400 rounded-full mt-1.5"></div>
                  <div className="flex-1">
                    <p className="font-medium text-gray-900">Transaction créée</p>
                    <p className="text-sm text-gray-500">{formatDateTime(transaction.created_at)}</p>
                  </div>
                </div>
              )}
              
              {transaction.paid_at && (
                <div className="flex items-start space-x-3">
                  <div className="w-2 h-2 bg-blue-400 rounded-full mt-1.5"></div>
                  <div className="flex-1">
                    <p className="font-medium text-gray-900">Paiement effectué</p>
                    <p className="text-sm text-gray-500">{formatDateTime(transaction.paid_at)}</p>
                  </div>
                </div>
              )}
              
              {transaction.confirmed_at && (
                <div className="flex items-start space-x-3">
                  <div className="w-2 h-2 bg-indigo-400 rounded-full mt-1.5"></div>
                  <div className="flex-1">
                    <p className="font-medium text-gray-900">Transaction confirmée</p>
                    <p className="text-sm text-gray-500">{formatDateTime(transaction.confirmed_at)}</p>
                  </div>
                </div>
              )}
              
              {transaction.completed_at && (
                <div className="flex items-start space-x-3">
                  <div className="w-2 h-2 bg-green-400 rounded-full mt-1.5"></div>
                  <div className="flex-1">
                    <p className="font-medium text-gray-900">Transaction complétée</p>
                    <p className="text-sm text-gray-500">{formatDateTime(transaction.completed_at)}</p>
                  </div>
                </div>
              )}
              
              {transaction.cancelled_at && (
                <div className="flex items-start space-x-3">
                  <div className="w-2 h-2 bg-red-400 rounded-full mt-1.5"></div>
                  <div className="flex-1">
                    <p className="font-medium text-gray-900">Transaction annulée</p>
                    <p className="text-sm text-gray-500">{formatDateTime(transaction.cancelled_at)}</p>
                    {transaction.cancelled_reason && (
                      <p className="text-sm text-gray-600 mt-1">
                        Raison: {transaction.cancelled_reason}
                      </p>
                    )}
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Sidebar Actions */}
        <div className="space-y-6">
          {/* Actions */}
          <div className="bg-white rounded-xl shadow-sm p-6">
            <h3 className="text-lg font-semibold text-gray-900 mb-4">
              Actions
            </h3>
            
            <div className="space-y-3">
              {transaction.status === 'confirmed' && (
                <button
                  onClick={() => completeMutation.mutate()}
                  disabled={completeMutation.isPending}
                  className="w-full px-4 py-3 bg-gradient-to-r from-green-500 to-green-600 text-white rounded-lg hover:from-green-600 hover:to-green-700 disabled:opacity-50"
                >
                  {completeMutation.isPending ? 'Traitement...' : 'Marquer comme complété'}
                </button>
              )}
              
              {(transaction.status === 'pending' || transaction.status === 'paid') && (
                <button
                  onClick={() => {
                    if (confirm('Êtes-vous sûr de vouloir annuler cette transaction ?')) {
                      cancelMutation.mutate('Annulé par le client');
                    }
                  }}
                  disabled={cancelMutation.isPending}
                  className="w-full px-4 py-3 bg-red-600 text-white rounded-lg hover:bg-red-700 disabled:opacity-50"
                >
                  {cancelMutation.isPending ? 'Annulation...' : 'Annuler la transaction'}
                </button>
              )}
              
              <button className="w-full px-4 py-3 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200">
                <Download className="h-5 w-5 inline mr-2" />
                Télécharger le reçu
              </button>
              
              <button className="w-full px-4 py-3 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200">
                Signaler un problème
              </button>
            </div>
          </div>

          {/* Support */}
          <div className="bg-blue-50 border border-blue-200 rounded-xl p-6">
            <h4 className="font-semibold text-blue-900 mb-2">
              Besoin d'aide ?
            </h4>
            <p className="text-sm text-blue-700 mb-4">
              Notre équipe est disponible 24/7 pour vous assister.
            </p>
            <button className="w-full px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700">
              Contacter le support
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
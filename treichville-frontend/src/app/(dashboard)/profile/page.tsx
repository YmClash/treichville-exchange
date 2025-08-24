'use client';

import { useState } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { 
  User, Phone, Mail, Shield, Key, Bell, Smartphone,
  Check, X, Edit2, Save, Camera, AlertCircle, Loader2
} from 'lucide-react';
import { useAuthStore } from '@/lib/stores/authStore';
import { authApi } from '@/lib/api/auth';
import { UpdateProfileInput, ChangePasswordInput, updateProfileSchema, changePasswordSchema } from '@/lib/utils/validators';
import { formatPhone, formatDateTime } from '@/lib/utils/formatters';
import { KYC_LEVELS, USER_ROLES } from '@/lib/utils/constants';
import { toast } from '@/lib/hooks/useToast';

export default function ProfilePage() {
  const queryClient = useQueryClient();
  const { user, setUser } = useAuthStore();
  const [isEditingProfile, setIsEditingProfile] = useState(false);
  const [isChangingPassword, setIsChangingPassword] = useState(false);
  const [isEnabling2FA, setIsEnabling2FA] = useState(false);

  // Profile form
  const {
    register: registerProfile,
    handleSubmit: handleSubmitProfile,
    formState: { errors: profileErrors },
    reset: resetProfile,
  } = useForm<UpdateProfileInput>({
    resolver: zodResolver(updateProfileSchema),
    defaultValues: {
      name: user?.name || '',
      email: user?.email || '',
      phone: user?.phone || '',
    },
  });

  // Password form
  const {
    register: registerPassword,
    handleSubmit: handleSubmitPassword,
    formState: { errors: passwordErrors },
    reset: resetPassword,
  } = useForm<ChangePasswordInput>({
    resolver: zodResolver(changePasswordSchema),
  });

  // Update profile mutation
  const updateProfileMutation = useMutation({
    mutationFn: authApi.updateProfile,
    onSuccess: (data) => {
      setUser(data);
      setIsEditingProfile(false);
      toast({
        title: 'Profil mis à jour avec succès',
        variant: 'success',
      });
    },
    onError: (error: any) => {
      toast({
        title: 'Erreur',
        description: error.message || 'Impossible de mettre à jour le profil',
        variant: 'destructive',
      });
    },
  });

  // Change password mutation
  const changePasswordMutation = useMutation({
    mutationFn: authApi.changePassword,
    onSuccess: () => {
      setIsChangingPassword(false);
      resetPassword();
      toast({
        title: 'Mot de passe modifié avec succès',
        variant: 'success',
      });
    },
    onError: (error: any) => {
      toast({
        title: 'Erreur',
        description: error.message || 'Impossible de modifier le mot de passe',
        variant: 'destructive',
      });
    },
  });

  // Enable 2FA mutation
  const enable2FAMutation = useMutation({
    mutationFn: authApi.enable2FA,
    onSuccess: (data) => {
      setIsEnabling2FA(true);
      toast({
        title: '2FA activé',
        description: 'Scannez le QR code avec votre application d\'authentification',
      });
    },
  });

  const onSubmitProfile = (data: UpdateProfileInput) => {
    updateProfileMutation.mutate(data);
  };

  const onSubmitPassword = (data: ChangePasswordInput) => {
    changePasswordMutation.mutate(data);
  };

  const getKYCColor = (level: string) => {
    switch(level) {
      case 'level2': return 'text-green-600 bg-green-100';
      case 'level1': return 'text-blue-600 bg-blue-100';
      case 'level0': return 'text-yellow-600 bg-yellow-100';
      default: return 'text-gray-600 bg-gray-100';
    }
  };

  return (
    <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      {/* Header */}
      <div className="mb-8">
        <h1 className="text-3xl font-bold text-gray-900">Mon Profil</h1>
        <p className="text-gray-600 mt-1">
          Gérez vos informations personnelles et paramètres de sécurité
        </p>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
        {/* Profile Card */}
        <div className="lg:col-span-1">
          <div className="bg-white rounded-xl shadow-sm p-6">
            <div className="text-center">
              {/* Avatar */}
              <div className="relative inline-block mb-4">
                <div className="w-32 h-32 bg-gradient-to-br from-orange-400 to-green-500 rounded-full flex items-center justify-center">
                  <span className="text-white text-4xl font-bold">
                    {user?.name?.split(' ').map(n => n[0]).join('').toUpperCase()}
                  </span>
                </div>
                <button className="absolute bottom-0 right-0 p-2 bg-white rounded-full shadow-lg hover:bg-gray-100">
                  <Camera className="h-5 w-5 text-gray-600" />
                </button>
              </div>

              {/* Name & Role */}
              <h2 className="text-xl font-semibold text-gray-900">{user?.name}</h2>
              <div className="mt-2 inline-flex items-center px-3 py-1 rounded-full text-sm font-medium bg-orange-100 text-orange-700">
                {USER_ROLES[user?.role as keyof typeof USER_ROLES]?.icon} {USER_ROLES[user?.role as keyof typeof USER_ROLES]?.label}
              </div>

              {/* KYC Status */}
              <div className="mt-4 space-y-2">
                <div className={`inline-flex items-center px-3 py-1 rounded-full text-sm ${getKYCColor(user?.kyc_status || 'none')}`}>
                  <Shield className="h-4 w-4 mr-1" />
                  {KYC_LEVELS[user?.kyc_status as keyof typeof KYC_LEVELS]?.label}
                </div>
                {user?.kyc_status !== 'level2' && (
                  <button className="text-sm text-orange-600 hover:text-orange-700 font-medium">
                    Améliorer mon statut →
                  </button>
                )}
              </div>

              {/* Stats */}
              <div className="mt-6 pt-6 border-t space-y-3 text-left">
                <div className="flex justify-between text-sm">
                  <span className="text-gray-500">Membre depuis</span>
                  <span className="font-medium text-gray-900">
                    {new Date(user?.created_at || '').toLocaleDateString('fr-FR', { month: 'long', year: 'numeric' })}
                  </span>
                </div>
                <div className="flex justify-between text-sm">
                  <span className="text-gray-500">Dernière connexion</span>
                  <span className="font-medium text-gray-900">
                    {formatDateTime(user?.last_login_at || new Date().toISOString())}
                  </span>
                </div>
                <div className="flex justify-between text-sm">
                  <span className="text-gray-500">Statut</span>
                  <span className={`font-medium ${user?.is_active ? 'text-green-600' : 'text-red-600'}`}>
                    {user?.is_active ? 'Actif' : 'Inactif'}
                  </span>
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Settings */}
        <div className="lg:col-span-2 space-y-6">
          {/* Personal Information */}
          <div className="bg-white rounded-xl shadow-sm p-6">
            <div className="flex items-center justify-between mb-6">
              <h3 className="text-lg font-semibold text-gray-900">
                Informations Personnelles
              </h3>
              {!isEditingProfile ? (
                <button
                  onClick={() => setIsEditingProfile(true)}
                  className="text-orange-600 hover:text-orange-700"
                >
                  <Edit2 className="h-5 w-5" />
                </button>
              ) : (
                <button
                  onClick={() => {
                    setIsEditingProfile(false);
                    resetProfile();
                  }}
                  className="text-gray-600 hover:text-gray-700"
                >
                  <X className="h-5 w-5" />
                </button>
              )}
            </div>

            {!isEditingProfile ? (
              <div className="space-y-4">
                <div className="flex items-center space-x-3">
                  <User className="h-5 w-5 text-gray-400" />
                  <div>
                    <p className="text-sm text-gray-500">Nom complet</p>
                    <p className="font-medium text-gray-900">{user?.name}</p>
                  </div>
                </div>
                <div className="flex items-center space-x-3">
                  <Phone className="h-5 w-5 text-gray-400" />
                  <div>
                    <p className="text-sm text-gray-500">Téléphone</p>
                    <p className="font-medium text-gray-900">{formatPhone(user?.phone || '')}</p>
                  </div>
                </div>
                <div className="flex items-center space-x-3">
                  <Mail className="h-5 w-5 text-gray-400" />
                  <div>
                    <p className="text-sm text-gray-500">Email</p>
                    <p className="font-medium text-gray-900">{user?.email || 'Non renseigné'}</p>
                  </div>
                </div>
              </div>
            ) : (
              <form onSubmit={handleSubmitProfile(onSubmitProfile)} className="space-y-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    Nom complet
                  </label>
                  <input
                    {...registerProfile('name')}
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-orange-500"
                  />
                  {profileErrors.name && (
                    <p className="text-red-500 text-sm mt-1">{profileErrors.name.message}</p>
                  )}
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    Email
                  </label>
                  <input
                    {...registerProfile('email')}
                    type="email"
                    className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-orange-500"
                  />
                  {profileErrors.email && (
                    <p className="text-red-500 text-sm mt-1">{profileErrors.email.message}</p>
                  )}
                </div>
                <div className="flex justify-end">
                  <button
                    type="submit"
                    disabled={updateProfileMutation.isPending}
                    className="flex items-center px-4 py-2 bg-gradient-to-r from-orange-500 to-orange-600 text-white rounded-lg hover:from-orange-600 hover:to-orange-700 disabled:opacity-50"
                  >
                    {updateProfileMutation.isPending ? (
                      <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                    ) : (
                      <Save className="h-4 w-4 mr-2" />
                    )}
                    Enregistrer
                  </button>
                </div>
              </form>
            )}
          </div>

          {/* Security Settings */}
          <div className="bg-white rounded-xl shadow-sm p-6">
            <h3 className="text-lg font-semibold text-gray-900 mb-6">
              Sécurité
            </h3>

            <div className="space-y-4">
              {/* Change Password */}
              <div className="border rounded-lg p-4">
                <div className="flex items-center justify-between">
                  <div className="flex items-center space-x-3">
                    <Key className="h-5 w-5 text-gray-400" />
                    <div>
                      <p className="font-medium text-gray-900">Mot de passe</p>
                      <p className="text-sm text-gray-500">Dernière modification il y a 30 jours</p>
                    </div>
                  </div>
                  <button
                    onClick={() => setIsChangingPassword(!isChangingPassword)}
                    className="text-orange-600 hover:text-orange-700 text-sm font-medium"
                  >
                    Modifier
                  </button>
                </div>

                {isChangingPassword && (
                  <form onSubmit={handleSubmitPassword(onSubmitPassword)} className="mt-4 space-y-3">
                    <div>
                      <input
                        {...registerPassword('current_password')}
                        type="password"
                        placeholder="Mot de passe actuel"
                        className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-orange-500"
                      />
                      {passwordErrors.current_password && (
                        <p className="text-red-500 text-sm mt-1">{passwordErrors.current_password.message}</p>
                      )}
                    </div>
                    <div>
                      <input
                        {...registerPassword('new_password')}
                        type="password"
                        placeholder="Nouveau mot de passe"
                        className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-orange-500"
                      />
                      {passwordErrors.new_password && (
                        <p className="text-red-500 text-sm mt-1">{passwordErrors.new_password.message}</p>
                      )}
                    </div>
                    <div>
                      <input
                        {...registerPassword('new_password_confirmation')}
                        type="password"
                        placeholder="Confirmer le nouveau mot de passe"
                        className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-orange-500"
                      />
                      {passwordErrors.new_password_confirmation && (
                        <p className="text-red-500 text-sm mt-1">{passwordErrors.new_password_confirmation.message}</p>
                      )}
                    </div>
                    <div className="flex justify-end space-x-2">
                      <button
                        type="button"
                        onClick={() => {
                          setIsChangingPassword(false);
                          resetPassword();
                        }}
                        className="px-4 py-2 text-gray-700 bg-gray-100 rounded-lg hover:bg-gray-200"
                      >
                        Annuler
                      </button>
                      <button
                        type="submit"
                        disabled={changePasswordMutation.isPending}
                        className="px-4 py-2 bg-gradient-to-r from-orange-500 to-orange-600 text-white rounded-lg hover:from-orange-600 hover:to-orange-700 disabled:opacity-50"
                      >
                        {changePasswordMutation.isPending ? 'Modification...' : 'Modifier'}
                      </button>
                    </div>
                  </form>
                )}
              </div>

              {/* 2FA */}
              <div className="border rounded-lg p-4">
                <div className="flex items-center justify-between">
                  <div className="flex items-center space-x-3">
                    <Smartphone className="h-5 w-5 text-gray-400" />
                    <div>
                      <p className="font-medium text-gray-900">Authentification à deux facteurs</p>
                      <p className="text-sm text-gray-500">
                        {user?.two_fa_enabled ? 'Activé' : 'Ajoutez une couche de sécurité supplémentaire'}
                      </p>
                    </div>
                  </div>
                  {user?.two_fa_enabled ? (
                    <span className="text-green-600">
                      <Check className="h-5 w-5" />
                    </span>
                  ) : (
                    <button
                      onClick={() => enable2FAMutation.mutate()}
                      className="text-orange-600 hover:text-orange-700 text-sm font-medium"
                    >
                      Activer
                    </button>
                  )}
                </div>
              </div>

              {/* Notifications */}
              <div className="border rounded-lg p-4">
                <div className="flex items-center justify-between">
                  <div className="flex items-center space-x-3">
                    <Bell className="h-5 w-5 text-gray-400" />
                    <div>
                      <p className="font-medium text-gray-900">Notifications</p>
                      <p className="text-sm text-gray-500">Gérer vos préférences de notification</p>
                    </div>
                  </div>
                  <button className="text-orange-600 hover:text-orange-700 text-sm font-medium">
                    Configurer
                  </button>
                </div>
              </div>
            </div>
          </div>

          {/* Danger Zone */}
          <div className="bg-red-50 border border-red-200 rounded-xl p-6">
            <h3 className="text-lg font-semibold text-red-900 mb-4">
              Zone Dangereuse
            </h3>
            <div className="flex items-center justify-between">
              <div>
                <p className="font-medium text-red-900">Supprimer mon compte</p>
                <p className="text-sm text-red-700">Cette action est irréversible</p>
              </div>
              <button className="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700">
                Supprimer
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
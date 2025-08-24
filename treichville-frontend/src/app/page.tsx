'use client';

import { useState, useEffect } from 'react';
import Link from 'next/link';
import { useRouter } from 'next/navigation';
import { 
  ArrowRight, TrendingUp, Shield, Zap, Globe, 
  Users, Clock, CreditCard, Smartphone, CheckCircle,
  Menu, X, ChevronDown, Star, Award, BarChart3, Activity
} from 'lucide-react';
import { motion } from 'framer-motion';

export default function LandingPage() {
  const router = useRouter();
  const [isMenuOpen, setIsMenuOpen] = useState(false);
  const [scrolled, setScrolled] = useState(false);

  useEffect(() => {
    const handleScroll = () => {
      setScrolled(window.scrollY > 20);
    };
    window.addEventListener('scroll', handleScroll);
    return () => window.removeEventListener('scroll', handleScroll);
  }, []);

  const features = [
    {
      icon: <TrendingUp className="h-8 w-8" />,
      title: "Meilleurs Taux",
      description: "Accédez aux taux de change les plus compétitifs du marché de Treichville"
    },
    {
      icon: <Shield className="h-8 w-8" />,
      title: "100% Sécurisé",
      description: "Transactions protégées par cryptage de niveau bancaire et authentification 2FA"
    },
    {
      icon: <Zap className="h-8 w-8" />,
      title: "Ultra Rapide",
      description: "Échangez vos devises en moins de 5 minutes, 24h/24 et 7j/7"
    },
    {
      icon: <Globe className="h-8 w-8" />,
      title: "Multi-Devises",
      description: "XOF, EUR, USD, GBP et bientôt Bitcoin - tout en un seul endroit"
    },
    {
      icon: <Users className="h-8 w-8" />,
      title: "200+ Changeurs",
      description: "Réseau de changeurs vérifiés de la Rue 12 à votre service"
    },
    {
      icon: <Smartphone className="h-8 w-8" />,
      title: "Mobile First",
      description: "Application optimisée pour mobile avec mode hors ligne"
    }
  ];

  const stats = [
    { value: "500M+", label: "FCFA échangés/jour" },
    { value: "200+", label: "Changeurs actifs" },
    { value: "10K+", label: "Clients satisfaits" },
    { value: "99.9%", label: "Disponibilité" }
  ];

  const testimonials = [
    {
      name: "Kouassi Yao",
      role: "Entrepreneur",
      content: "Treichville Exchange a révolutionné mes transactions internationales. Je gagne un temps fou!",
      rating: 5
    },
    {
      name: "Aminata Diallo",
      role: "Commerçante",
      content: "Les taux sont vraiment compétitifs et le service est rapide. Je recommande vivement!",
      rating: 5
    },
    {
      name: "Mohamed Koné",
      role: "Changeur",
      content: "La plateforme m'a permis d'augmenter ma clientèle de 300%. C'est une vraie révolution!",
      rating: 5
    }
  ];

  return (
    <div className="min-h-screen bg-gradient-to-b from-white to-gray-50">
      {/* Header */}
      <header className={`fixed top-0 left-0 right-0 z-50 transition-all duration-300 ${
        scrolled ? 'bg-white/95 backdrop-blur-md shadow-md' : 'bg-transparent'
      }`}>
        <nav className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex items-center justify-between h-16 sm:h-20">
            {/* Logo */}
            <div className="flex items-center">
              <div className="flex items-center space-x-3">
                <div className="w-10 h-10 bg-gradient-to-br from-orange-500 to-green-600 rounded-xl flex items-center justify-center">
                  <span className="text-white font-bold text-lg">TE</span>
                </div>
                <div>
                  <h1 className="text-xl font-bold text-gray-900">Treichville</h1>
                  <p className="text-xs text-gray-600 -mt-1">Exchange</p>
                </div>
              </div>
            </div>

            {/* Desktop Navigation */}
            <div className="hidden md:flex items-center space-x-8">
              <a href="#features" className="text-gray-700 hover:text-orange-600 transition">
                Fonctionnalités
              </a>
              <a href="#how-it-works" className="text-gray-700 hover:text-orange-600 transition">
                Comment ça marche
              </a>
              <a href="#testimonials" className="text-gray-700 hover:text-orange-600 transition">
                Témoignages
              </a>
              <Link href="/register" className="text-gray-700 hover:text-orange-600 transition">
                Devenir Changeur
              </Link>
              <Link href="/health" className="text-gray-700 hover:text-orange-600 transition">
                <Activity className="h-5 w-5" />
              </Link>
              <Link 
                href="/login"
                className="px-6 py-2 bg-gradient-to-r from-orange-500 to-orange-600 text-white rounded-lg hover:from-orange-600 hover:to-orange-700 transition"
              >
                Se connecter
              </Link>
            </div>

            {/* Mobile menu button */}
            <button
              onClick={() => setIsMenuOpen(!isMenuOpen)}
              className="md:hidden p-2 text-gray-700"
            >
              {isMenuOpen ? <X className="h-6 w-6" /> : <Menu className="h-6 w-6" />}
            </button>
          </div>

          {/* Mobile Navigation */}
          {isMenuOpen && (
            <div className="md:hidden py-4 bg-white rounded-b-2xl shadow-lg">
              <div className="flex flex-col space-y-4 px-4">
                <a href="#features" className="text-gray-700 hover:text-orange-600 transition">
                  Fonctionnalités
                </a>
                <a href="#how-it-works" className="text-gray-700 hover:text-orange-600 transition">
                  Comment ça marche
                </a>
                <a href="#testimonials" className="text-gray-700 hover:text-orange-600 transition">
                  Témoignages
                </a>
                <Link href="/register" className="text-gray-700 hover:text-orange-600 transition">
                  Devenir Changeur
                </Link>
                <Link href="/health" className="text-gray-700 hover:text-orange-600 transition flex items-center">
                  <Activity className="h-4 w-4 mr-2" />
                  System Status
                </Link>
                <Link 
                  href="/login"
                  className="px-6 py-3 bg-gradient-to-r from-orange-500 to-orange-600 text-white rounded-lg text-center hover:from-orange-600 hover:to-orange-700 transition"
                >
                  Se connecter
                </Link>
              </div>
            </div>
          )}
        </nav>
      </header>

      {/* Hero Section */}
      <section className="relative pt-32 pb-20 px-4 sm:px-6 lg:px-8 overflow-hidden">
        {/* Background Pattern */}
        <div className="absolute inset-0 -z-10">
          <div className="absolute top-0 left-0 w-96 h-96 bg-orange-100 rounded-full filter blur-3xl opacity-30 -translate-x-1/2 -translate-y-1/2"></div>
          <div className="absolute bottom-0 right-0 w-96 h-96 bg-green-100 rounded-full filter blur-3xl opacity-30 translate-x-1/2 translate-y-1/2"></div>
        </div>

        <div className="max-w-7xl mx-auto">
          <div className="grid lg:grid-cols-2 gap-12 items-center">
            {/* Left Content */}
            <div>
              <motion.div
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.5 }}
              >
                <div className="inline-flex items-center px-4 py-2 bg-orange-100 rounded-full text-orange-700 text-sm font-medium mb-6">
                  <Award className="h-4 w-4 mr-2" />
                  #1 Plateforme de change en Côte d'Ivoire
                </div>
                
                <h1 className="text-4xl sm:text-5xl lg:text-6xl font-bold text-gray-900 mb-6">
                  Échangez vos devises en{' '}
                  <span className="text-transparent bg-clip-text bg-gradient-to-r from-orange-500 to-green-600">
                    toute confiance
                  </span>
                </h1>
                
                <p className="text-xl text-gray-600 mb-8">
                  Connectez-vous aux meilleurs changeurs de la Rue 12, Treichville. 
                  Taux compétitifs, transactions sécurisées, service rapide.
                </p>

                <div className="flex flex-col sm:flex-row gap-4">
                  <Link
                    href="/login"
                    className="inline-flex items-center justify-center px-8 py-4 bg-gradient-to-r from-orange-500 to-orange-600 text-white text-lg font-semibold rounded-xl hover:from-orange-600 hover:to-orange-700 transition shadow-lg hover:shadow-xl"
                  >
                    Commencer maintenant
                    <ArrowRight className="ml-2 h-5 w-5" />
                  </Link>
                  <Link
                    href="/register"
                    className="inline-flex items-center justify-center px-8 py-4 bg-white text-gray-900 text-lg font-semibold rounded-xl border-2 border-gray-200 hover:border-orange-500 transition"
                  >
                    Créer un compte
                  </Link>
                </div>

                <div className="mt-8 flex items-center space-x-6">
                  <div className="flex -space-x-2">
                    {[1,2,3,4].map((i) => (
                      <div key={i} className="w-10 h-10 bg-gradient-to-br from-orange-400 to-green-500 rounded-full border-2 border-white" />
                    ))}
                  </div>
                  <div>
                    <div className="flex items-center">
                      {[1,2,3,4,5].map((i) => (
                        <Star key={i} className="h-4 w-4 text-yellow-500 fill-current" />
                      ))}
                    </div>
                    <p className="text-sm text-gray-600">10,000+ utilisateurs actifs</p>
                  </div>
                </div>
              </motion.div>
            </div>

            {/* Right Content - Phone Mockup */}
            <motion.div
              initial={{ opacity: 0, x: 20 }}
              animate={{ opacity: 1, x: 0 }}
              transition={{ duration: 0.5, delay: 0.2 }}
              className="relative"
            >
              <div className="relative mx-auto w-80 h-[600px] bg-gray-900 rounded-[3rem] shadow-2xl">
                <div className="absolute inset-x-0 top-0 h-6 bg-gray-900 rounded-t-[3rem]"></div>
                <div className="absolute inset-2 bg-white rounded-[2.5rem] overflow-hidden">
                  {/* Mock App Screen */}
                  <div className="p-6">
                    <div className="flex items-center justify-between mb-6">
                      <div className="text-sm font-medium text-gray-900">Solde disponible</div>
                      <div className="text-xs text-gray-500">Voir tout →</div>
                    </div>
                    <div className="text-3xl font-bold text-gray-900 mb-8">
                      2,500,000 FCFA
                    </div>
                    
                    <div className="space-y-4">
                      <div className="p-4 bg-gradient-to-r from-orange-50 to-green-50 rounded-xl">
                        <div className="flex items-center justify-between mb-2">
                          <span className="text-sm text-gray-600">EUR → XOF</span>
                          <span className="text-sm font-bold text-green-600">656.50</span>
                        </div>
                        <div className="text-xs text-gray-500">Meilleur taux disponible</div>
                      </div>
                      
                      <div className="grid grid-cols-2 gap-3">
                        <button className="p-3 bg-orange-500 text-white rounded-xl text-sm font-medium">
                          Acheter
                        </button>
                        <button className="p-3 bg-green-600 text-white rounded-xl text-sm font-medium">
                          Vendre
                        </button>
                      </div>
                      
                      <div className="pt-4">
                        <div className="text-sm font-medium text-gray-900 mb-3">Transactions récentes</div>
                        {[1,2,3].map((i) => (
                          <div key={i} className="flex items-center justify-between py-2">
                            <div>
                              <div className="text-sm font-medium text-gray-900">EUR → XOF</div>
                              <div className="text-xs text-gray-500">Il y a 2h</div>
                            </div>
                            <div className="text-sm font-bold text-green-600">+150,000 FCFA</div>
                          </div>
                        ))}
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </motion.div>
          </div>
        </div>
      </section>

      {/* Stats Section */}
      <section className="py-16 bg-gradient-to-r from-orange-500 to-green-600">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="grid grid-cols-2 md:grid-cols-4 gap-8">
            {stats.map((stat, index) => (
              <motion.div
                key={index}
                initial={{ opacity: 0, y: 20 }}
                whileInView={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.5, delay: index * 0.1 }}
                className="text-center"
              >
                <div className="text-3xl sm:text-4xl font-bold text-white mb-2">
                  {stat.value}
                </div>
                <div className="text-white/80 text-sm">
                  {stat.label}
                </div>
              </motion.div>
            ))}
          </div>
        </div>
      </section>

      {/* Features Section */}
      <section id="features" className="py-20 px-4 sm:px-6 lg:px-8">
        <div className="max-w-7xl mx-auto">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5 }}
            className="text-center mb-16"
          >
            <h2 className="text-3xl sm:text-4xl font-bold text-gray-900 mb-4">
              Pourquoi choisir Treichville Exchange?
            </h2>
            <p className="text-xl text-gray-600 max-w-3xl mx-auto">
              La première plateforme qui digitalise le marché informel de change de devises en Afrique de l'Ouest
            </p>
          </motion.div>

          <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-8">
            {features.map((feature, index) => (
              <motion.div
                key={index}
                initial={{ opacity: 0, y: 20 }}
                whileInView={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.5, delay: index * 0.1 }}
                className="bg-white p-6 rounded-2xl shadow-lg hover:shadow-xl transition-shadow"
              >
                <div className="w-14 h-14 bg-gradient-to-br from-orange-500 to-green-600 rounded-xl flex items-center justify-center text-white mb-4">
                  {feature.icon}
                </div>
                <h3 className="text-xl font-semibold text-gray-900 mb-2">
                  {feature.title}
                </h3>
                <p className="text-gray-600">
                  {feature.description}
                </p>
              </motion.div>
            ))}
          </div>
        </div>
      </section>

      {/* How It Works Section */}
      <section id="how-it-works" className="py-20 px-4 sm:px-6 lg:px-8 bg-gray-50">
        <div className="max-w-7xl mx-auto">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5 }}
            className="text-center mb-16"
          >
            <h2 className="text-3xl sm:text-4xl font-bold text-gray-900 mb-4">
              Comment ça marche?
            </h2>
            <p className="text-xl text-gray-600">
              Échangez vos devises en 3 étapes simples
            </p>
          </motion.div>

          <div className="grid md:grid-cols-3 gap-8">
            {[
              {
                step: "1",
                title: "Créez votre compte",
                description: "Inscription rapide avec votre numéro de téléphone. Vérification instantanée.",
                icon: <Users className="h-8 w-8" />
              },
              {
                step: "2",
                title: "Choisissez votre taux",
                description: "Comparez les taux en temps réel et sélectionnez le meilleur changeur.",
                icon: <BarChart3 className="h-8 w-8" />
              },
              {
                step: "3",
                title: "Effectuez l'échange",
                description: "Payez en cash ou mobile money et recevez vos devises instantanément.",
                icon: <CheckCircle className="h-8 w-8" />
              }
            ].map((item, index) => (
              <motion.div
                key={index}
                initial={{ opacity: 0, y: 20 }}
                whileInView={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.5, delay: index * 0.1 }}
                className="relative"
              >
                <div className="bg-white p-6 rounded-2xl shadow-lg">
                  <div className="w-12 h-12 bg-gradient-to-br from-orange-500 to-green-600 rounded-full flex items-center justify-center text-white font-bold text-xl mb-4">
                    {item.step}
                  </div>
                  <h3 className="text-xl font-semibold text-gray-900 mb-2">
                    {item.title}
                  </h3>
                  <p className="text-gray-600">
                    {item.description}
                  </p>
                </div>
                {index < 2 && (
                  <div className="hidden md:block absolute top-1/2 -right-4 transform -translate-y-1/2">
                    <ArrowRight className="h-8 w-8 text-orange-500" />
                  </div>
                )}
              </motion.div>
            ))}
          </div>
        </div>
      </section>

      {/* Testimonials Section */}
      <section id="testimonials" className="py-20 px-4 sm:px-6 lg:px-8">
        <div className="max-w-7xl mx-auto">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5 }}
            className="text-center mb-16"
          >
            <h2 className="text-3xl sm:text-4xl font-bold text-gray-900 mb-4">
              Ce que disent nos utilisateurs
            </h2>
            <p className="text-xl text-gray-600">
              Plus de 10,000 clients satisfaits nous font confiance
            </p>
          </motion.div>

          <div className="grid md:grid-cols-3 gap-8">
            {testimonials.map((testimonial, index) => (
              <motion.div
                key={index}
                initial={{ opacity: 0, y: 20 }}
                whileInView={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.5, delay: index * 0.1 }}
                className="bg-white p-6 rounded-2xl shadow-lg"
              >
                <div className="flex mb-4">
                  {[...Array(testimonial.rating)].map((_, i) => (
                    <Star key={i} className="h-5 w-5 text-yellow-500 fill-current" />
                  ))}
                </div>
                <p className="text-gray-600 mb-4 italic">
                  "{testimonial.content}"
                </p>
                <div className="flex items-center">
                  <div className="w-12 h-12 bg-gradient-to-br from-orange-400 to-green-500 rounded-full mr-3"></div>
                  <div>
                    <div className="font-semibold text-gray-900">
                      {testimonial.name}
                    </div>
                    <div className="text-sm text-gray-500">
                      {testimonial.role}
                    </div>
                  </div>
                </div>
              </motion.div>
            ))}
          </div>
        </div>
      </section>

      {/* CTA Section */}
      <section className="py-20 px-4 sm:px-6 lg:px-8 bg-gradient-to-r from-orange-500 to-green-600">
        <div className="max-w-4xl mx-auto text-center">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5 }}
          >
            <h2 className="text-3xl sm:text-4xl font-bold text-white mb-4">
              Prêt à révolutionner vos échanges de devises?
            </h2>
            <p className="text-xl text-white/90 mb-8">
              Rejoignez plus de 10,000 utilisateurs qui font confiance à Treichville Exchange
            </p>
            <div className="flex flex-col sm:flex-row gap-4 justify-center">
              <Link
                href="/register"
                className="inline-flex items-center justify-center px-8 py-4 bg-white text-orange-600 text-lg font-semibold rounded-xl hover:bg-gray-100 transition shadow-lg"
              >
                Créer un compte gratuit
                <ArrowRight className="ml-2 h-5 w-5" />
              </Link>
              <Link
                href="/login"
                className="inline-flex items-center justify-center px-8 py-4 bg-transparent text-white text-lg font-semibold rounded-xl border-2 border-white hover:bg-white/10 transition"
              >
                Se connecter
              </Link>
            </div>
          </motion.div>
        </div>
      </section>

      {/* Footer */}
      <footer className="bg-gray-900 text-white py-12 px-4 sm:px-6 lg:px-8">
        <div className="max-w-7xl mx-auto">
          <div className="grid md:grid-cols-4 gap-8">
            <div>
              <div className="flex items-center space-x-3 mb-4">
                <div className="w-10 h-10 bg-gradient-to-br from-orange-500 to-green-600 rounded-xl flex items-center justify-center">
                  <span className="text-white font-bold text-lg">TE</span>
                </div>
                <div>
                  <h3 className="text-xl font-bold">Treichville</h3>
                  <p className="text-xs text-gray-400 -mt-1">Exchange</p>
                </div>
              </div>
              <p className="text-gray-400 text-sm">
                La première plateforme de change de devises digitale en Afrique de l'Ouest.
              </p>
            </div>
            
            <div>
              <h4 className="font-semibold mb-4">Produit</h4>
              <ul className="space-y-2 text-gray-400 text-sm">
                <li><a href="#features" className="hover:text-white transition">Fonctionnalités</a></li>
                <li><a href="#" className="hover:text-white transition">Tarifs</a></li>
                <li><a href="#" className="hover:text-white transition">API</a></li>
                <li><a href="#" className="hover:text-white transition">Sécurité</a></li>
              </ul>
            </div>
            
            <div>
              <h4 className="font-semibold mb-4">Entreprise</h4>
              <ul className="space-y-2 text-gray-400 text-sm">
                <li><a href="#" className="hover:text-white transition">À propos</a></li>
                <li><a href="#" className="hover:text-white transition">Blog</a></li>
                <li><a href="#" className="hover:text-white transition">Carrières</a></li>
                <li><a href="#" className="hover:text-white transition">Contact</a></li>
              </ul>
            </div>
            
            <div>
              <h4 className="font-semibold mb-4">Légal</h4>
              <ul className="space-y-2 text-gray-400 text-sm">
                <li><a href="#" className="hover:text-white transition">Conditions d'utilisation</a></li>
                <li><a href="#" className="hover:text-white transition">Politique de confidentialité</a></li>
                <li><a href="#" className="hover:text-white transition">Cookies</a></li>
                <li><a href="#" className="hover:text-white transition">Licences</a></li>
              </ul>
            </div>
          </div>
          
          <div className="border-t border-gray-800 mt-8 pt-8 text-center text-gray-400 text-sm">
            <p>&copy; 2025 Treichville Exchange. Tous droits réservés.</p>
            <p className="mt-2">Made with ❤️ in Abidjan, Côte d'Ivoire 🇨🇮</p>
          </div>
        </div>
      </footer>
    </div>
  );
}
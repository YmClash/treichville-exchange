import type { Metadata,Viewport } from 'next';
import { Inter } from 'next/font/google';
import './globals.css';
import { QueryProvider } from '@/components/providers/QueryProvider';
import { ServiceWorkerProvider } from '@/components/providers/ServiceWorkerProvider';

const inter = Inter({ subsets: ['latin'] });

export const viewport: Viewport = {
  width: 'device-width',
  initialScale: 1,
  themeColor: '#FF6B00',
}

export const metadata: Metadata = {
  title: {
    template: '%s | Treichville Exchange',
    default: 'Treichville Exchange',
  },
  description: 'Plateforme de change de devises pour l\'Afrique de l\'Ouest',
  keywords: ['change', 'devises', 'Côte d\'Ivoire', 'Abidjan', 'Treichville', 'XOF', 'EUR', 'USD'],
  authors: [{ name: 'Treichville Exchange Team' }],
  creator: 'Treichville Exchange',
  publisher: 'Treichville Exchange',
  formatDetection: {
    email: false,
    address: false,
    telephone: false,
  },
  // viewport: {
  //   width: 'device-width',
  //   initialScale: 1,
  //   maximumScale: 1,
  // },
  // themeColor: [
  //   { media: '(prefers-color-scheme: light)', color: '#FF6B00' },
  //   { media: '(prefers-color-scheme: dark)', color: '#FF6B00' },
  // ],
  icons: {
    icon: [
      { url: '/favicon.ico' },
      { url: '/icons/favicon-16x16.png', sizes: '16x16', type: 'image/png' },
      { url: '/icons/favicon-32x32.png', sizes: '32x32', type: 'image/png' },
    ],
    apple: [
      { url: '/icons/apple-touch-icon-120x120.png', sizes: '120x120', type: 'image/png' },
      { url: '/icons/apple-touch-icon-180x180.png', sizes: '180x180', type: 'image/png' },
    ],
  },
  manifest: '/manifest.json',
  openGraph: {
    type: 'website',
    locale: 'fr_CI',
    url: 'https://treichville-exchange.com',
    siteName: 'Treichville Exchange',
    title: 'Treichville Exchange',
    description: 'Plateforme de change de devises pour l\'Afrique de l\'Ouest',
    images: [
      {
        url: '/og-image.png',
        width: 1200,
        height: 630,
        alt: 'Treichville Exchange',
      },
    ],
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Treichville Exchange',
    description: 'Plateforme de change de devises pour l\'Afrique de l\'Ouest',
    images: ['/twitter-image.png'],
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="fr" suppressHydrationWarning>
      <body className={`${inter.className} antialiased`}>
        <ServiceWorkerProvider>
          <QueryProvider>
            {children}
          </QueryProvider>
        </ServiceWorkerProvider>
      </body>
    </html>
  );
}

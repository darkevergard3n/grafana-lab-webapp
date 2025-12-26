/**
 * =============================================================================
 * ROOT LAYOUT
 * =============================================================================
 * This is the root layout for the Next.js application.
 * It wraps all pages and provides common elements.
 * =============================================================================
 */

import type { Metadata } from 'next';
import { Inter } from 'next/font/google';
import './globals.css';

// Load Inter font from Google Fonts
const inter = Inter({ subsets: ['latin'] });

// Page metadata
export const metadata: Metadata = {
  title: 'OrderFlow - Enterprise Order Management',
  description: 'Enterprise-grade Order Management System for Grafana Observability Lab',
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body className={inter.className}>
        {children}
      </body>
    </html>
  );
}

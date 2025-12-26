/**
 * =============================================================================
 * MAIN DASHBOARD PAGE
 * =============================================================================
 * Enterprise-grade dashboard for the Order Management System.
 * 
 * This page displays:
 * - KPI cards (orders, revenue, pending, alerts)
 * - Order trend chart
 * - Recent orders table
 * - System health status
 * =============================================================================
 */

'use client';

import React, { useState, useEffect } from 'react';
import { 
  ShoppingCart, 
  DollarSign, 
  Clock, 
  AlertTriangle,
  Package,
  Users,
  CreditCard,
  Bell,
  Settings,
  BarChart3,
  TrendingUp,
  TrendingDown,
  CheckCircle,
  XCircle,
  Menu,
  Search,
  Moon,
  Sun
} from 'lucide-react';

// =============================================================================
// TYPE DEFINITIONS
// =============================================================================

interface Order {
  id: string;
  customer: string;
  email: string;
  status: 'pending' | 'processing' | 'shipped' | 'delivered' | 'cancelled';
  amount: number;
  items: number;
  date: string;
}

interface ServiceHealth {
  name: string;
  status: 'healthy' | 'unhealthy' | 'unknown';
  latency: number;
}

// =============================================================================
// MOCK DATA (Replace with API calls in production)
// =============================================================================

const mockOrders: Order[] = [
  { id: 'ORD-001', customer: 'PT Telkom Indonesia', email: 'procurement@telkom.co.id', status: 'shipped', amount: 15420000, items: 12, date: '2024-01-15' },
  { id: 'ORD-002', customer: 'Bank BCA', email: 'it@bca.co.id', status: 'pending', amount: 8750000, items: 5, date: '2024-01-15' },
  { id: 'ORD-003', customer: 'Gojek', email: 'ops@gojek.com', status: 'processing', amount: 23100000, items: 18, date: '2024-01-14' },
  { id: 'ORD-004', customer: 'Tokopedia', email: 'warehouse@tokopedia.com', status: 'delivered', amount: 4200000, items: 3, date: '2024-01-14' },
  { id: 'ORD-005', customer: 'Pertamina', email: 'supply@pertamina.com', status: 'pending', amount: 67800000, items: 45, date: '2024-01-13' },
];

const mockServices: ServiceHealth[] = [
  { name: 'Order Service', status: 'healthy', latency: 45 },
  { name: 'Inventory Service', status: 'healthy', latency: 32 },
  { name: 'Payment Service', status: 'healthy', latency: 78 },
  { name: 'User Service', status: 'healthy', latency: 51 },
  { name: 'Notification Service', status: 'healthy', latency: 23 },
];

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

function formatCurrency(amount: number): string {
  return new Intl.NumberFormat('id-ID', {
    style: 'currency',
    currency: 'IDR',
    minimumFractionDigits: 0,
  }).format(amount);
}

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString('id-ID', {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
  });
}

// =============================================================================
// COMPONENTS
// =============================================================================

// KPI Card Component
function KPICard({ 
  title, 
  value, 
  change, 
  changeType, 
  icon: Icon 
}: { 
  title: string; 
  value: string; 
  change: string; 
  changeType: 'up' | 'down' | 'neutral';
  icon: React.ElementType;
}) {
  return (
    <div className="bg-white dark:bg-slate-800 rounded-xl p-6 shadow-sm border border-slate-200 dark:border-slate-700">
      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm text-slate-500 dark:text-slate-400">{title}</p>
          <p className="text-2xl font-bold text-slate-900 dark:text-white mt-1">{value}</p>
          <div className="flex items-center mt-2">
            {changeType === 'up' && <TrendingUp className="w-4 h-4 text-emerald-500 mr-1" />}
            {changeType === 'down' && <TrendingDown className="w-4 h-4 text-red-500 mr-1" />}
            <span className={`text-sm ${
              changeType === 'up' ? 'text-emerald-500' : 
              changeType === 'down' ? 'text-red-500' : 'text-slate-500'
            }`}>
              {change}
            </span>
          </div>
        </div>
        <div className="p-3 bg-blue-50 dark:bg-blue-900/20 rounded-lg">
          <Icon className="w-6 h-6 text-blue-600 dark:text-blue-400" />
        </div>
      </div>
    </div>
  );
}

// Status Badge Component
function StatusBadge({ status }: { status: Order['status'] }) {
  const styles = {
    pending: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400',
    processing: 'bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-400',
    shipped: 'bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-400',
    delivered: 'bg-emerald-100 text-emerald-800 dark:bg-emerald-900/30 dark:text-emerald-400',
    cancelled: 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400',
  };

  return (
    <span className={`px-2.5 py-1 rounded-full text-xs font-medium ${styles[status]}`}>
      {status.charAt(0).toUpperCase() + status.slice(1)}
    </span>
  );
}

// Service Health Indicator
function ServiceHealthCard({ service }: { service: ServiceHealth }) {
  return (
    <div className="flex items-center justify-between py-3 border-b border-slate-100 dark:border-slate-700 last:border-0">
      <div className="flex items-center gap-3">
        {service.status === 'healthy' ? (
          <CheckCircle className="w-5 h-5 text-emerald-500" />
        ) : (
          <XCircle className="w-5 h-5 text-red-500" />
        )}
        <span className="text-sm text-slate-700 dark:text-slate-300">{service.name}</span>
      </div>
      <span className="text-sm text-slate-500">{service.latency}ms</span>
    </div>
  );
}

// Sidebar Navigation
function Sidebar({ isOpen, setIsOpen }: { isOpen: boolean; setIsOpen: (open: boolean) => void }) {
  const navItems = [
    { icon: BarChart3, label: 'Dashboard', active: true },
    { icon: ShoppingCart, label: 'Orders', active: false },
    { icon: Package, label: 'Inventory', active: false },
    { icon: CreditCard, label: 'Payments', active: false },
    { icon: Users, label: 'Users', active: false },
    { icon: Bell, label: 'Notifications', active: false },
    { icon: Settings, label: 'Settings', active: false },
  ];

  return (
    <aside className={`fixed inset-y-0 left-0 z-50 w-64 bg-slate-900 transform transition-transform duration-200 ease-in-out ${
      isOpen ? 'translate-x-0' : '-translate-x-full'
    } lg:translate-x-0`}>
      <div className="flex items-center gap-3 px-6 py-5 border-b border-slate-800">
        <div className="p-2 bg-blue-600 rounded-lg">
          <ShoppingCart className="w-5 h-5 text-white" />
        </div>
        <span className="text-lg font-bold text-white">OrderFlow</span>
      </div>
      
      <nav className="mt-6 px-4">
        {navItems.map((item) => (
          <a
            key={item.label}
            href="#"
            className={`flex items-center gap-3 px-4 py-3 rounded-lg mb-1 transition-colors ${
              item.active
                ? 'bg-blue-600 text-white'
                : 'text-slate-400 hover:bg-slate-800 hover:text-white'
            }`}
          >
            <item.icon className="w-5 h-5" />
            <span>{item.label}</span>
          </a>
        ))}
      </nav>
      
      <div className="absolute bottom-0 left-0 right-0 p-4 border-t border-slate-800">
        <div className="flex items-center gap-3 px-4 py-3">
          <div className="w-10 h-10 rounded-full bg-slate-700 flex items-center justify-center">
            <span className="text-sm font-medium text-white">AD</span>
          </div>
          <div>
            <p className="text-sm font-medium text-white">Admin User</p>
            <p className="text-xs text-slate-400">admin@company.com</p>
          </div>
        </div>
      </div>
    </aside>
  );
}

// =============================================================================
// MAIN DASHBOARD COMPONENT
// =============================================================================

export default function Dashboard() {
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [darkMode, setDarkMode] = useState(false);
  const [orders] = useState<Order[]>(mockOrders);
  const [services] = useState<ServiceHealth[]>(mockServices);

  // Toggle dark mode
  useEffect(() => {
    if (darkMode) {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
  }, [darkMode]);

  return (
    <div className="min-h-screen bg-slate-50 dark:bg-slate-900">
      {/* Sidebar */}
      <Sidebar isOpen={sidebarOpen} setIsOpen={setSidebarOpen} />
      
      {/* Main Content */}
      <div className="lg:pl-64">
        {/* Header */}
        <header className="sticky top-0 z-40 bg-white dark:bg-slate-800 border-b border-slate-200 dark:border-slate-700">
          <div className="flex items-center justify-between px-6 py-4">
            <div className="flex items-center gap-4">
              <button
                onClick={() => setSidebarOpen(!sidebarOpen)}
                className="lg:hidden p-2 rounded-lg hover:bg-slate-100 dark:hover:bg-slate-700"
              >
                <Menu className="w-5 h-5 text-slate-600 dark:text-slate-300" />
              </button>
              
              <div className="relative">
                <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
                <input
                  type="text"
                  placeholder="Search orders, customers..."
                  className="pl-10 pr-4 py-2 w-64 bg-slate-100 dark:bg-slate-700 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>
            </div>
            
            <div className="flex items-center gap-3">
              <button
                onClick={() => setDarkMode(!darkMode)}
                className="p-2 rounded-lg hover:bg-slate-100 dark:hover:bg-slate-700"
              >
                {darkMode ? (
                  <Sun className="w-5 h-5 text-slate-400" />
                ) : (
                  <Moon className="w-5 h-5 text-slate-600" />
                )}
              </button>
              
              <button className="relative p-2 rounded-lg hover:bg-slate-100 dark:hover:bg-slate-700">
                <Bell className="w-5 h-5 text-slate-600 dark:text-slate-300" />
                <span className="absolute top-1 right-1 w-2 h-2 bg-red-500 rounded-full"></span>
              </button>
            </div>
          </div>
        </header>
        
        {/* Page Content */}
        <main className="p-6">
          {/* Page Title */}
          <div className="mb-8">
            <h1 className="text-2xl font-bold text-slate-900 dark:text-white">Dashboard</h1>
            <p className="text-slate-500 dark:text-slate-400 mt-1">
              Welcome back! Here&apos;s what&apos;s happening with your orders.
            </p>
          </div>
          
          {/* KPI Cards */}
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
            <KPICard
              title="Total Orders"
              value="1,247"
              change="+12% from last month"
              changeType="up"
              icon={ShoppingCart}
            />
            <KPICard
              title="Revenue"
              value={formatCurrency(892500000)}
              change="+8.1% from last month"
              changeType="up"
              icon={DollarSign}
            />
            <KPICard
              title="Pending Orders"
              value="23"
              change="-5 from yesterday"
              changeType="down"
              icon={Clock}
            />
            <KPICard
              title="Low Stock Alerts"
              value="12"
              change="Requires attention"
              changeType="neutral"
              icon={AlertTriangle}
            />
          </div>
          
          {/* Charts and Tables Row */}
          <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 mb-8">
            {/* Recent Orders Table */}
            <div className="lg:col-span-2 bg-white dark:bg-slate-800 rounded-xl shadow-sm border border-slate-200 dark:border-slate-700">
              <div className="px-6 py-4 border-b border-slate-200 dark:border-slate-700 flex items-center justify-between">
                <h2 className="font-semibold text-slate-900 dark:text-white">Recent Orders</h2>
                <a href="#" className="text-sm text-blue-600 hover:text-blue-700">View all →</a>
              </div>
              <div className="overflow-x-auto">
                <table className="w-full">
                  <thead>
                    <tr className="border-b border-slate-200 dark:border-slate-700">
                      <th className="text-left px-6 py-3 text-xs font-medium text-slate-500 uppercase tracking-wider">Order ID</th>
                      <th className="text-left px-6 py-3 text-xs font-medium text-slate-500 uppercase tracking-wider">Customer</th>
                      <th className="text-left px-6 py-3 text-xs font-medium text-slate-500 uppercase tracking-wider">Status</th>
                      <th className="text-right px-6 py-3 text-xs font-medium text-slate-500 uppercase tracking-wider">Amount</th>
                      <th className="text-left px-6 py-3 text-xs font-medium text-slate-500 uppercase tracking-wider">Date</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-slate-200 dark:divide-slate-700">
                    {orders.map((order) => (
                      <tr key={order.id} className="hover:bg-slate-50 dark:hover:bg-slate-700/50">
                        <td className="px-6 py-4">
                          <span className="font-medium text-slate-900 dark:text-white">#{order.id}</span>
                        </td>
                        <td className="px-6 py-4">
                          <div>
                            <p className="text-sm text-slate-900 dark:text-white">{order.customer}</p>
                            <p className="text-xs text-slate-500">{order.email}</p>
                          </div>
                        </td>
                        <td className="px-6 py-4">
                          <StatusBadge status={order.status} />
                        </td>
                        <td className="px-6 py-4 text-right">
                          <span className="font-medium text-slate-900 dark:text-white">
                            {formatCurrency(order.amount)}
                          </span>
                        </td>
                        <td className="px-6 py-4 text-slate-500">
                          {formatDate(order.date)}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
            
            {/* System Health */}
            <div className="bg-white dark:bg-slate-800 rounded-xl shadow-sm border border-slate-200 dark:border-slate-700">
              <div className="px-6 py-4 border-b border-slate-200 dark:border-slate-700">
                <h2 className="font-semibold text-slate-900 dark:text-white">System Health</h2>
              </div>
              <div className="px-6 py-4">
                {services.map((service) => (
                  <ServiceHealthCard key={service.name} service={service} />
                ))}
              </div>
              <div className="px-6 py-4 border-t border-slate-200 dark:border-slate-700">
                <a href="#" className="text-sm text-blue-600 hover:text-blue-700">
                  View Grafana Dashboard →
                </a>
              </div>
            </div>
          </div>
          
          {/* Footer */}
          <footer className="text-center text-sm text-slate-500 mt-12">
            <p>Order Management System v1.0.0 | Grafana Observability Lab</p>
            <p className="mt-1">Built for learning enterprise monitoring with Prometheus, Loki, and Tempo</p>
          </footer>
        </main>
      </div>
      
      {/* Sidebar Overlay (mobile) */}
      {sidebarOpen && (
        <div
          className="fixed inset-0 bg-black/50 z-40 lg:hidden"
          onClick={() => setSidebarOpen(false)}
        />
      )}
    </div>
  );
}

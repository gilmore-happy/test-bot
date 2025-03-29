import React from 'react';
import { Link, useLocation } from 'react-router-dom';
import { useUIStore } from '../../store/uiStore';
import { useSystemStore } from '../../store/systemStore';
import clsx from 'clsx';

// Import icons
import {
  HomeIcon,
  ChartBarIcon,
  CurrencyDollarIcon,
  Cog6ToothIcon,
  ArrowsRightLeftIcon,
  BanknotesIcon,
  ChevronDoubleLeftIcon,
  ChevronDoubleRightIcon,
} from '@heroicons/react/24/outline';

interface NavItemProps {
  to: string;
  icon: React.ReactNode;
  label: string;
  active: boolean;
  collapsed: boolean;
}

const NavItem: React.FC<NavItemProps> = ({ to, icon, label, active, collapsed }) => {
  return (
    <Link
      to={to}
      className={clsx(
        'flex items-center px-4 py-3 rounded-lg transition-colors duration-200',
        active
          ? 'bg-primary-700 text-white'
          : 'text-secondary-300 hover:bg-secondary-700 hover:text-white'
      )}
    >
      <div className="w-6 h-6">{icon}</div>
      {!collapsed && <span className="ml-3">{label}</span>}
    </Link>
  );
};

const Sidebar: React.FC = () => {
  const location = useLocation();
  const { sidebarCollapsed, toggleSidebar } = useUIStore();
  const { status } = useSystemStore();

  const navItems = [
    { to: '/', icon: <HomeIcon />, label: 'Overview' },
    { to: '/performance', icon: <ChartBarIcon />, label: 'Performance' },
    { to: '/positions', icon: <ArrowsRightLeftIcon />, label: 'Positions' },
    { to: '/strategies', icon: <CurrencyDollarIcon />, label: 'Strategies' },
    { to: '/assets', icon: <BanknotesIcon />, label: 'Assets' },
    { to: '/settings', icon: <Cog6ToothIcon />, label: 'Settings' },
  ];

  return (
    <div
      className={clsx(
        'h-screen bg-secondary-800 border-r border-secondary-700 flex flex-col transition-all duration-300',
        sidebarCollapsed ? 'w-16' : 'w-64'
      )}
    >
      <div className="flex items-center justify-between p-4 border-b border-secondary-700">
        {!sidebarCollapsed && (
          <div className="text-xl font-bold text-white">Solana HFT</div>
        )}
        <button
          onClick={toggleSidebar}
          className="p-1 rounded-md text-secondary-400 hover:text-white hover:bg-secondary-700"
        >
          {sidebarCollapsed ? (
            <ChevronDoubleRightIcon className="w-5 h-5" />
          ) : (
            <ChevronDoubleLeftIcon className="w-5 h-5" />
          )}
        </button>
      </div>

      <div className="flex-1 py-4 overflow-y-auto">
        <nav className="px-2 space-y-1">
          {navItems.map((item) => (
            <NavItem
              key={item.to}
              to={item.to}
              icon={item.icon}
              label={item.label}
              active={location.pathname === item.to}
              collapsed={sidebarCollapsed}
            />
          ))}
        </nav>
      </div>

      <div className="p-4 border-t border-secondary-700">
        <div className={clsx('flex items-center', sidebarCollapsed ? 'justify-center' : 'justify-between')}>
          <div
            className={clsx(
              'w-3 h-3 rounded-full',
              status?.status === 'online'
                ? 'bg-success-500'
                : status?.status === 'degraded'
                ? 'bg-warning-500'
                : 'bg-danger-500'
            )}
          />
          {!sidebarCollapsed && (
            <span className="text-sm text-secondary-400">
              {status?.status === 'online'
                ? 'System Online'
                : status?.status === 'degraded'
                ? 'System Degraded'
                : 'System Offline'}
            </span>
          )}
        </div>
      </div>
    </div>
  );
};

export default Sidebar;
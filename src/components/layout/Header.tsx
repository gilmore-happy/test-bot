import React, { useState, useEffect } from 'react';
import { useNotificationStore } from '../../store/notificationStore';
import { useUIStore } from '../../store/uiStore';
import { useAssetStore } from '../../store/assetStore';
import { format } from 'date-fns';
import {
  BellIcon,
  SunIcon,
  MoonIcon,
  ArrowPathIcon,
} from '@heroicons/react/24/outline';

const Header: React.FC = () => {
  const [currentTime, setCurrentTime] = useState(new Date());
  const { unreadCount, toggleNotificationCenter } = useNotificationStore();
  const { theme, toggleTheme } = useUIStore();
  const { assets } = useAssetStore();

  // Calculate total balance
  const totalBalance = assets.reduce((sum, asset) => sum + asset.valueUsd, 0);

  // Update time every second
  useEffect(() => {
    const timer = setInterval(() => {
      setCurrentTime(new Date());
    }, 1000);

    return () => {
      clearInterval(timer);
    };
  }, []);

  return (
    <header className="bg-secondary-800 border-b border-secondary-700 py-2 px-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-4">
          <div className="text-sm text-secondary-400">
            {format(currentTime, 'EEEE, MMMM d, yyyy')}
          </div>
          <div className="text-sm font-mono text-secondary-300">
            {format(currentTime, 'HH:mm:ss')}
          </div>
        </div>

        <div className="flex items-center space-x-6">
          <div className="text-right">
            <div className="text-sm text-secondary-400">Total Balance</div>
            <div className="text-lg font-semibold text-white">
              ${totalBalance.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
            </div>
          </div>

          <div className="flex items-center space-x-2">
            <button
              onClick={toggleTheme}
              className="p-2 rounded-full text-secondary-400 hover:text-white hover:bg-secondary-700"
              aria-label={theme === 'dark' ? 'Switch to light mode' : 'Switch to dark mode'}
            >
              {theme === 'dark' ? <SunIcon className="w-5 h-5" /> : <MoonIcon className="w-5 h-5" />}
            </button>

            <button
              className="p-2 rounded-full text-secondary-400 hover:text-white hover:bg-secondary-700"
              aria-label="Refresh data"
              onClick={() => window.location.reload()}
            >
              <ArrowPathIcon className="w-5 h-5" />
            </button>

            <button
              onClick={toggleNotificationCenter}
              className="p-2 rounded-full text-secondary-400 hover:text-white hover:bg-secondary-700 relative"
              aria-label="Open notifications"
            >
              <BellIcon className="w-5 h-5" />
              {unreadCount > 0 && (
                <span className="absolute top-0 right-0 inline-flex items-center justify-center w-4 h-4 text-xs font-bold text-white bg-danger-500 rounded-full">
                  {unreadCount > 9 ? '9+' : unreadCount}
                </span>
              )}
            </button>
          </div>
        </div>
      </div>
    </header>
  );
};

export default Header;
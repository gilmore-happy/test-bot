                                                                  import React, { useState } from 'react';
import { useUIStore } from '../store/uiStore';
import { useSystemStore } from '../store/systemStore';
import { useNotificationStore } from '../store/notificationStore';
import {
  Cog6ToothIcon,
  MoonIcon,
  SunIcon,
  BellIcon,
  ServerIcon,
  ShieldCheckIcon,
  ArrowPathIcon,
  ComputerDesktopIcon,
} from '@heroicons/react/24/outline';
import HardwareConfigPanel from '../components/settings/HardwareConfigPanel';
import clsx from 'clsx';

const SettingsSection: React.FC<{
  title: string;
  icon: React.ReactNode;
  children: React.ReactNode;
}> = ({ title, icon, children }) => {
  return (
    <div className="card mb-6">
      <div className="flex items-center mb-4">
        <div className="p-2 rounded-full bg-secondary-700 mr-3">
          {icon}
        </div>
        <h2 className="text-xl font-semibold">{title}</h2>
      </div>
      <div className="space-y-4">
        {children}
      </div>
    </div>
  );
};

const SettingRow: React.FC<{
  title: string;
  description?: string;
  children: React.ReactNode;
}> = ({ title, description, children }) => {
  return (
    <div className="flex items-center justify-between py-3 border-b border-secondary-700 last:border-0">
      <div>
        <h3 className="font-medium">{title}</h3>
        {description && (
          <p className="text-sm text-secondary-400">{description}</p>
        )}
      </div>
      <div>
        {children}
      </div>
    </div>
  );
};

const Settings: React.FC = () => {
  const { theme, toggleTheme, refreshRate, setRefreshRate } = useUIStore();
  const { status } = useSystemStore();
  const { addNotification } = useNotificationStore();
  
  const [notificationsEnabled, setNotificationsEnabled] = useState(true);
  const [soundEnabled, setSoundEnabled] = useState(true);
  const [autoLogout, setAutoLogout] = useState(30);
  const [apiEndpoint, setApiEndpoint] = useState('https://api.example.com');
  const [wsEndpoint, setWsEndpoint] = useState('wss://ws.example.com');
  
  const handleRestart = () => {
    if (window.confirm('Are you sure you want to restart the system? This will temporarily interrupt trading.')) {
      // API call to restart system would go here
      addNotification({
        type: 'info',
        title: 'System Restart',
        message: 'System restart initiated. This may take a few moments.',
      });
    }
  };
  
  const handleSaveEndpoints = () => {
    // API call to save endpoints would go here
    addNotification({
      type: 'success',
      title: 'Settings Saved',
      message: 'API and WebSocket endpoints have been updated.',
    });
  };
  
  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold mb-6">Settings</h1>
      
      <SettingsSection title="Hardware Configuration" icon={<ComputerDesktopIcon className="w-5 h-5 text-primary-500" />}>
        <HardwareConfigPanel />
      </SettingsSection>
      
      <SettingsSection title="Appearance" icon={<Cog6ToothIcon className="w-5 h-5 text-primary-500" />}>
        <SettingRow 
          title="Theme" 
          description="Toggle between light and dark mode"
        >
          <button
            onClick={toggleTheme}
            className="flex items-center px-3 py-2 rounded-md bg-secondary-700 hover:bg-secondary-600"
          >
            {theme === 'dark' ? (
              <>
                <SunIcon className="w-5 h-5 mr-2 text-warning-500" />
                <span>Light Mode</span>
              </>
            ) : (
              <>
                <MoonIcon className="w-5 h-5 mr-2 text-primary-500" />
                <span>Dark Mode</span>
              </>
            )}
          </button>
        </SettingRow>
        
        <SettingRow 
          title="Data Refresh Rate" 
          description="How often to refresh data automatically (in seconds)"
        >
          <select
            value={refreshRate}
            onChange={(e) => setRefreshRate(Number(e.target.value))}
            className="input"
            aria-label="Select refresh rate"
            title="Select refresh rate"
          >
            <option value={1}>1 second</option>
            <option value={5}>5 seconds</option>
            <option value={10}>10 seconds</option>
            <option value={30}>30 seconds</option>
            <option value={60}>1 minute</option>
          </select>
        </SettingRow>
      </SettingsSection>
      
      <SettingsSection title="Notifications" icon={<BellIcon className="w-5 h-5 text-primary-500" />}>
        <SettingRow 
          title="Enable Notifications" 
          description="Receive alerts for important events"
        >
          <div className="relative inline-block w-12 h-6 transition duration-200 ease-in-out rounded-full">
            <input
              type="checkbox"
              className="absolute w-6 h-6 opacity-0 z-10 cursor-pointer"
              checked={notificationsEnabled}
              onChange={() => setNotificationsEnabled(!notificationsEnabled)}
              aria-label="Enable notifications"
              title="Enable notifications"
            />
            <div className={clsx(
              "w-12 h-6 transition-colors duration-200 ease-in-out rounded-full",
              notificationsEnabled ? "bg-primary-600" : "bg-secondary-600"
            )}></div>
            <div className={clsx(
              "absolute top-0.5 left-0.5 bg-white w-5 h-5 transition-transform duration-200 ease-in-out rounded-full",
              notificationsEnabled ? "transform translate-x-6" : ""
            )}></div>
          </div>
        </SettingRow>
        
        <SettingRow 
          title="Sound Alerts" 
          description="Play sound for critical notifications"
        >
          <div className="relative inline-block w-12 h-6 transition duration-200 ease-in-out rounded-full">
            <input
              type="checkbox"
              className="absolute w-6 h-6 opacity-0 z-10 cursor-pointer"
              checked={soundEnabled}
              onChange={() => setSoundEnabled(!soundEnabled)}
              aria-label="Enable sound alerts"
              title="Enable sound alerts"
            />
            <div className={clsx(
              "w-12 h-6 transition-colors duration-200 ease-in-out rounded-full",
              soundEnabled ? "bg-primary-600" : "bg-secondary-600"
            )}></div>
            <div className={clsx(
              "absolute top-0.5 left-0.5 bg-white w-5 h-5 transition-transform duration-200 ease-in-out rounded-full",
              soundEnabled ? "transform translate-x-6" : ""
            )}></div>
          </div>
        </SettingRow>
      </SettingsSection>
      
      <SettingsSection title="Security" icon={<ShieldCheckIcon className="w-5 h-5 text-primary-500" />}>
        <SettingRow 
          title="Auto Logout" 
          description="Automatically log out after period of inactivity (minutes)"
        >
          <select
            value={autoLogout}
            onChange={(e) => setAutoLogout(Number(e.target.value))}
            className="input"
            aria-label="Select auto logout time"
            title="Select auto logout time"
          >
            <option value={5}>5 minutes</option>
            <option value={15}>15 minutes</option>
            <option value={30}>30 minutes</option>
            <option value={60}>1 hour</option>
            <option value={0}>Never</option>
          </select>
        </SettingRow>
      </SettingsSection>
      
      <SettingsSection title="System" icon={<ServerIcon className="w-5 h-5 text-primary-500" />}>
        <SettingRow 
          title="API Endpoint" 
          description="URL for API requests"
        >
          <div className="flex">
            <label className="sr-only" htmlFor="api-endpoint">API Endpoint URL</label>
            <input
              id="api-endpoint"
              type="text"
              value={apiEndpoint}
              onChange={(e) => setApiEndpoint(e.target.value)}
              className="input mr-2"
              aria-label="API endpoint URL"
              placeholder="https://api.example.com"
            />
          </div>
        </SettingRow>
        
        <SettingRow 
          title="WebSocket Endpoint" 
          description="URL for real-time updates"
        >
          <div className="flex">
            <label className="sr-only" htmlFor="ws-endpoint">WebSocket Endpoint URL</label>
            <input
              id="ws-endpoint"
              type="text"
              value={wsEndpoint}
              onChange={(e) => setWsEndpoint(e.target.value)}
              className="input mr-2"
              aria-label="WebSocket endpoint URL"
              placeholder="wss://ws.example.com"
            />
          </div>
        </SettingRow>
        
        <div className="flex justify-end mt-4">
          <button
            onClick={handleSaveEndpoints}
            className="btn btn-primary"
          >
            Save Endpoints
          </button>
        </div>
        
        <div className="mt-6 pt-4 border-t border-secondary-700">
          <SettingRow 
            title="System Status" 
            description="Current system operational status"
          >
            <div className="flex items-center">
              <div className={clsx(
                'w-3 h-3 rounded-full mr-2',
                status?.status === 'online'
                  ? 'bg-success-500'
                  : status?.status === 'degraded'
                  ? 'bg-warning-500'
                  : 'bg-danger-500'
              )}></div>
              <span className="capitalize">
                {status?.status || 'Unknown'}
              </span>
            </div>
          </SettingRow>
          
          <SettingRow 
            title="System Version" 
            description="Current software version"
          >
            <span>{status?.version || 'Unknown'}</span>
          </SettingRow>
          
          <div className="flex justify-end mt-4">
            <button
              onClick={handleRestart}
              className="btn btn-danger flex items-center"
            >
              <ArrowPathIcon className="w-5 h-5 mr-2" />
              Restart System
            </button>
          </div>
        </div>
      </SettingsSection>
    </div>
  );
};

export default Settings;

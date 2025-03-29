import React from 'react';
import { Outlet } from 'react-router-dom';
import Sidebar from './Sidebar';
import Header from './Header';
import StatusBar from './StatusBar';
import NotificationCenter from './NotificationCenter';
import { useNotificationStore } from '../../store/notificationStore';

const Layout: React.FC = () => {
  const { showNotificationCenter } = useNotificationStore();

  return (
    <div className="flex h-screen bg-secondary-900 text-white">
      <Sidebar />
      
      <div className="flex flex-col flex-1 overflow-hidden">
        <Header />
        
        <main className="flex-1 overflow-auto p-6">
          <Outlet />
        </main>
        
        <StatusBar />
      </div>
      
      {showNotificationCenter && <NotificationCenter />}
    </div>
  );
};

export default Layout;
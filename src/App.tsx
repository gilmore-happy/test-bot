import React, { useEffect } from 'react';
import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom';
import Layout from './components/layout/Layout';
import { useSystemStore } from './store/systemStore';
import { useStrategyStore } from './store/strategyStore';
import { usePositionStore } from './store/positionStore';
import { useAssetStore } from './store/assetStore';
import { useWebSocket } from './api/websocket';
import { useNotificationStore } from './store/notificationStore';

// Import pages
import Overview from './pages/Overview';
import Performance from './pages/Performance';
import Positions from './pages/Positions';
import Strategies from './pages/Strategies';
import Assets from './pages/Assets';
import Settings from './pages/Settings';

const App: React.FC = () => {
  const { fetchStatus, fetchPerformanceMetrics } = useSystemStore();
  const { fetchStrategies } = useStrategyStore();
  const { fetchPositions } = usePositionStore();
  const { fetchAssets } = useAssetStore();
  const webSocket = useWebSocket();
  const { addNotification } = useNotificationStore();
  
  // Initialize data and WebSocket connection
  useEffect(() => {
    // Fetch initial data
    fetchStatus();
    fetchPerformanceMetrics();
    fetchStrategies();
    fetchPositions();
    fetchAssets();
    
    // Connect to WebSocket
    webSocket.connect();
    
    // Set up WebSocket event handlers
    webSocket.onConnect(() => {
      addNotification({
        type: 'success',
        title: 'WebSocket Connected',
        message: 'Real-time updates are now enabled.',
      });
    });
    
    webSocket.onDisconnect(() => {
      addNotification({
        type: 'warning',
        title: 'WebSocket Disconnected',
        message: 'Real-time updates are temporarily disabled. Reconnecting...',
      });
    });
    
    // Example of subscribing to WebSocket events
    webSocket.on('system_status', (data) => {
      fetchStatus();
    });
    
    webSocket.on('performance_update', (data) => {
      fetchPerformanceMetrics();
    });
    
    webSocket.on('strategy_update', (data) => {
      fetchStrategies();
    });
    
    webSocket.on('position_update', (data) => {
      fetchPositions();
    });
    
    webSocket.on('asset_update', (data) => {
      fetchAssets();
    });
    
    // Set up periodic data refresh
    const refreshInterval = setInterval(() => {
      fetchStatus();
      fetchPerformanceMetrics();
      fetchStrategies();
      fetchPositions();
      fetchAssets();
    }, 30000); // Refresh every 30 seconds
    
    // Clean up on unmount
    return () => {
      webSocket.disconnect();
      clearInterval(refreshInterval);
    };
  }, [
    fetchStatus,
    fetchPerformanceMetrics,
    fetchStrategies,
    fetchPositions,
    fetchAssets,
    webSocket,
    addNotification
  ]);
  
  return (
    <Router>
      <Routes>
        <Route path="/" element={<Layout />}>
          <Route index element={<Overview />} />
          <Route path="performance" element={<Performance />} />
          <Route path="positions" element={<Positions />} />
          <Route path="strategies" element={<Strategies />} />
          <Route path="assets" element={<Assets />} />
          <Route path="settings" element={<Settings />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Route>
      </Routes>
    </Router>
  );
};

export default App;
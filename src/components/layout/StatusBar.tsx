import React from 'react';
import { useSystemStore } from '../../store/systemStore';
import { useWebSocket } from '../../api/websocket';
import { WifiIcon, SignalIcon, ClockIcon } from '@heroicons/react/24/outline';
import clsx from 'clsx';

const StatusBar: React.FC = () => {
  const { performanceMetrics } = useSystemStore();
  const webSocket = useWebSocket();
  
  // Determine connection quality based on latency
  const getConnectionQuality = () => {
    if (!performanceMetrics?.latency.network) return 'unknown';
    
    const latency = performanceMetrics.latency.network;
    if (latency < 50) return 'excellent';
    if (latency < 100) return 'good';
    if (latency < 200) return 'fair';
    return 'poor';
  };
  
  const connectionQuality = getConnectionQuality();
  
  const getConnectionIcon = () => {
    switch (connectionQuality) {
      case 'excellent':
        return <SignalIcon className="w-4 h-4 text-success-500" />;
      case 'good':
        return <SignalIcon className="w-4 h-4 text-success-400" />;
      case 'fair':
        return <SignalIcon className="w-4 h-4 text-warning-500" />;
      case 'poor':
        return <SignalIcon className="w-4 h-4 text-danger-500" />;
      default:
        return <WifiIcon className="w-4 h-4 text-secondary-400" />;
    }
  };
  
  const getConnectionText = () => {
    switch (connectionQuality) {
      case 'excellent':
        return 'Excellent';
      case 'good':
        return 'Good';
      case 'fair':
        return 'Fair';
      case 'poor':
        return 'Poor';
      default:
        return 'Unknown';
    }
  };
  
  return (
    <div className="bg-secondary-800 border-t border-secondary-700 py-1 px-4 text-xs text-secondary-400">
      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-4">
          <div className="flex items-center space-x-1">
            {getConnectionIcon()}
            <span>Connection: {getConnectionText()}</span>
            {performanceMetrics?.latency.network && (
              <span className="text-secondary-500">
                ({Math.round(performanceMetrics.latency.network)}ms)
              </span>
            )}
          </div>
          
          <div className="flex items-center space-x-1">
            <ClockIcon className="w-4 h-4 text-secondary-500" />
            <span>
              Latency: {performanceMetrics ? `${Math.round(performanceMetrics.latency.average)}ms` : 'N/A'}
            </span>
          </div>
        </div>
        
        <div className="flex items-center space-x-4">
          <div>
            Success Rate: {performanceMetrics ? `${performanceMetrics.successRate.toFixed(1)}%` : 'N/A'}
          </div>
          
          <div>
            Throughput: {performanceMetrics ? `${performanceMetrics.throughput.toFixed(1)} tx/s` : 'N/A'}
          </div>
          
          <div className={clsx(
            'px-2 py-0.5 rounded-full',
            webSocket.isConnected()
              ? 'bg-success-900 text-success-300'
              : 'bg-danger-900 text-danger-300'
          )}>
            {webSocket.isConnected() ? 'Connected' : 'Disconnected'}
          </div>
        </div>
      </div>
    </div>
  );
};

export default StatusBar;
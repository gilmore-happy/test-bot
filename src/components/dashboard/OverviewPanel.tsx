import React from 'react';
import { useSystemStore } from '../../store/systemStore';
import { useStrategyStore } from '../../store/strategyStore';
import { Strategy } from '../../types';
import { 
  ArrowTrendingUpIcon, 
  ArrowTrendingDownIcon,
  ServerIcon,
  BoltIcon,
  ExclamationTriangleIcon,
  ArrowPathIcon,
  ClockIcon
} from '@heroicons/react/24/outline';
import clsx from 'clsx';

interface StatCardProps {
  title: string;
  value: string | number;
  icon: React.ReactNode;
  trend?: {
    value: number;
    isPositive: boolean;
  };
  status?: 'success' | 'warning' | 'danger' | 'neutral';
}

const StatCard: React.FC<StatCardProps> = ({ title, value, icon, trend, status = 'neutral' }) => {
  const getStatusColor = () => {
    switch (status) {
      case 'success':
        return 'border-success-500 bg-success-500/10';
      case 'warning':
        return 'border-warning-500 bg-warning-500/10';
      case 'danger':
        return 'border-danger-500 bg-danger-500/10';
      default:
        return 'border-secondary-700 bg-secondary-800';
    }
  };

  return (
    <div className={clsx(
      'rounded-lg border p-4 flex flex-col',
      getStatusColor()
    )}>
      <div className="flex justify-between items-start mb-2">
        <span className="text-sm text-secondary-400">{title}</span>
        <div className="p-2 rounded-full bg-secondary-800/50">{icon}</div>
      </div>
      <div className="text-2xl font-semibold mb-1">{value}</div>
      {trend && (
        <div className="flex items-center text-xs">
          {trend.isPositive ? (
            <ArrowTrendingUpIcon className="w-3 h-3 text-success-500 mr-1" />
          ) : (
            <ArrowTrendingDownIcon className="w-3 h-3 text-danger-500 mr-1" />
          )}
          <span className={trend.isPositive ? 'text-success-500' : 'text-danger-500'}>
            {trend.value}% {trend.isPositive ? 'increase' : 'decrease'}
          </span>
        </div>
      )}
    </div>
  );
};

const OverviewPanel: React.FC = () => {
  const { status, performanceMetrics } = useSystemStore();
  const { strategies } = useStrategyStore();
  
  const activeStrategies = strategies.filter((s: Strategy) => s.status === 'active').length;
  
  const getSystemStatusColor = () => {
    if (!status) return 'neutral';
    switch (status.status) {
      case 'online':
        return 'success';
      case 'degraded':
        return 'warning';
      case 'offline':
        return 'danger';
      default:
        return 'neutral';
    }
  };
  
  const formatUptime = (seconds: number) => {
    if (!seconds) return 'N/A';
    
    const days = Math.floor(seconds / 86400);
    const hours = Math.floor((seconds % 86400) / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    
    if (days > 0) {
      return `${days}d ${hours}h ${minutes}m`;
    }
    
    if (hours > 0) {
      return `${hours}h ${minutes}m`;
    }
    
    return `${minutes}m`;
  };
  
  return (
    <div className="card">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-semibold">System Overview</h2>
        <div className="flex space-x-2">
          <button className="btn btn-secondary text-sm py-1 px-3 flex items-center">
            <ArrowPathIcon className="w-4 h-4 mr-1" />
            Refresh
          </button>
          <button className="btn btn-danger text-sm py-1 px-3 flex items-center">
            <ExclamationTriangleIcon className="w-4 h-4 mr-1" />
            Emergency Stop
          </button>
        </div>
      </div>
      
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard
          title="System Status"
          value={status?.status ? status.status.charAt(0).toUpperCase() + status.status.slice(1) : 'Unknown'}
          icon={<ServerIcon className="w-5 h-5 text-secondary-400" />}
          status={getSystemStatusColor()}
        />
        
        <StatCard
          title="Active Strategies"
          value={activeStrategies}
          icon={<BoltIcon className="w-5 h-5 text-secondary-400" />}
          trend={strategies.length > 0 ? {
            value: Math.round((activeStrategies / strategies.length) * 100),
            isPositive: true
          } : undefined}
        />
        
        <StatCard
          title="Success Rate"
          value={performanceMetrics && performanceMetrics.successRate !== undefined ? `${performanceMetrics.successRate.toFixed(1)}%` : 'N/A'}
          icon={<ArrowTrendingUpIcon className="w-5 h-5 text-secondary-400" />}
          status={performanceMetrics && performanceMetrics.successRate !== undefined ?
                 (performanceMetrics.successRate > 95 ? 'success' :
                 performanceMetrics.successRate > 85 ? 'warning' : 'danger') : 'neutral'}
        />
        
        <StatCard
          title="Uptime"
          value={status ? formatUptime(status.uptime) : 'N/A'}
          icon={<ClockIcon className="w-5 h-5 text-secondary-400" />}
        />
      </div>
    </div>
  );
};

export default OverviewPanel;
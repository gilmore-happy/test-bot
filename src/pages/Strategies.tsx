import React, { useState } from 'react';
import { useStrategyStore } from '../store/strategyStore';
import { Strategy } from '../types';
import {
  PlayIcon,
  PauseIcon,
  StopIcon,
  PencilIcon,
  ChartBarIcon,
  ClockIcon,
} from '@heroicons/react/24/outline';
import clsx from 'clsx';

const StrategyCard: React.FC<{
  strategy: Strategy;
  onStatusChange: (id: string, status: 'active' | 'paused' | 'stopped') => void;
}> = ({ strategy, onStatusChange }) => {
  return (
    <div className="card">
      <div className="flex justify-between items-start mb-4">
        <div>
          <h3 className="text-lg font-semibold">{strategy.name}</h3>
          <p className="text-sm text-secondary-400">{strategy.description}</p>
        </div>
        <div className={clsx(
          'px-2 py-1 rounded-full text-xs font-medium',
          strategy.status === 'active' ? 'bg-success-100 text-success-800' :
          strategy.status === 'paused' ? 'bg-warning-100 text-warning-800' :
          'bg-secondary-100 text-secondary-800'
        )}>
          {strategy.status.charAt(0).toUpperCase() + strategy.status.slice(1)}
        </div>
      </div>
      
      <div className="grid grid-cols-2 gap-4 mb-4">
        <div>
          <div className="text-sm text-secondary-400 mb-1">Daily Performance</div>
          <div className={clsx(
            'text-lg font-semibold',
            strategy.performance.daily >= 0 ? 'text-success-500' : 'text-danger-500'
          )}>
            {strategy.performance.daily >= 0 ? '+' : ''}{strategy.performance.daily.toFixed(2)}%
          </div>
        </div>
        <div>
          <div className="text-sm text-secondary-400 mb-1">All-Time Performance</div>
          <div className={clsx(
            'text-lg font-semibold',
            strategy.performance.allTime >= 0 ? 'text-success-500' : 'text-danger-500'
          )}>
            {strategy.performance.allTime >= 0 ? '+' : ''}{strategy.performance.allTime.toFixed(2)}%
          </div>
        </div>
      </div>
      
      <div className="flex justify-between items-center">
        <div className="flex space-x-2">
          <button
            onClick={() => onStatusChange(strategy.id, 'active')}
            className={clsx(
              'p-2 rounded-full',
              strategy.status === 'active'
                ? 'bg-success-500 text-white'
                : 'bg-secondary-700 text-secondary-400 hover:text-white hover:bg-success-600'
            )}
            aria-label="Activate strategy"
            title="Activate"
          >
            <PlayIcon className="w-5 h-5" />
          </button>
          <button
            onClick={() => onStatusChange(strategy.id, 'paused')}
            className={clsx(
              'p-2 rounded-full',
              strategy.status === 'paused'
                ? 'bg-warning-500 text-white'
                : 'bg-secondary-700 text-secondary-400 hover:text-white hover:bg-warning-600'
            )}
            aria-label="Pause strategy"
            title="Pause"
          >
            <PauseIcon className="w-5 h-5" />
          </button>
          <button
            onClick={() => onStatusChange(strategy.id, 'stopped')}
            className={clsx(
              'p-2 rounded-full',
              strategy.status === 'stopped'
                ? 'bg-danger-500 text-white'
                : 'bg-secondary-700 text-secondary-400 hover:text-white hover:bg-danger-600'
            )}
            aria-label="Stop strategy"
            title="Stop"
          >
            <StopIcon className="w-5 h-5" />
          </button>
        </div>
        
        <div className="flex space-x-2">
          <button
            className="p-2 rounded-full bg-secondary-700 text-secondary-400 hover:text-white hover:bg-primary-600"
            aria-label="Edit strategy parameters"
            title="Edit Parameters"
          >
            <PencilIcon className="w-5 h-5" />
          </button>
          <button
            className="p-2 rounded-full bg-secondary-700 text-secondary-400 hover:text-white hover:bg-primary-600"
            aria-label="View strategy performance"
            title="View Performance"
          >
            <ChartBarIcon className="w-5 h-5" />
          </button>
          <button
            className="p-2 rounded-full bg-secondary-700 text-secondary-400 hover:text-white hover:bg-primary-600"
            aria-label="Configure schedule"
            title="Configure Schedule"
          >
            <ClockIcon className="w-5 h-5" />
          </button>
        </div>
      </div>
    </div>
  );
};

const Strategies: React.FC = () => {
  const { strategies, updateStatus } = useStrategyStore();
  const [filter, setFilter] = useState<'all' | 'active' | 'paused' | 'stopped'>('all');
  
  const filteredStrategies = filter === 'all'
    ? strategies
    : strategies.filter((s: Strategy) => s.status === filter);
  
  const handleStatusChange = (id: string, status: 'active' | 'paused' | 'stopped') => {
    updateStatus(id, status);
  };
  
  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h1 className="text-2xl font-bold">Strategy Control</h1>
        
        <div className="flex space-x-2">
          <button
            onClick={() => setFilter('all')}
            className={clsx(
              'px-3 py-1 rounded-md text-sm',
              filter === 'all'
                ? 'bg-primary-600 text-white'
                : 'bg-secondary-700 text-secondary-300 hover:bg-secondary-600'
            )}
          >
            All
          </button>
          <button
            onClick={() => setFilter('active')}
            className={clsx(
              'px-3 py-1 rounded-md text-sm',
              filter === 'active'
                ? 'bg-success-600 text-white'
                : 'bg-secondary-700 text-secondary-300 hover:bg-secondary-600'
            )}
          >
            Active
          </button>
          <button
            onClick={() => setFilter('paused')}
            className={clsx(
              'px-3 py-1 rounded-md text-sm',
              filter === 'paused'
                ? 'bg-warning-600 text-white'
                : 'bg-secondary-700 text-secondary-300 hover:bg-secondary-600'
            )}
          >
            Paused
          </button>
          <button
            onClick={() => setFilter('stopped')}
            className={clsx(
              'px-3 py-1 rounded-md text-sm',
              filter === 'stopped'
                ? 'bg-danger-600 text-white'
                : 'bg-secondary-700 text-secondary-300 hover:bg-secondary-600'
            )}
          >
            Stopped
          </button>
        </div>
      </div>
      
      <div className="card mb-6">
        <h2 className="text-xl font-semibold mb-4">Strategy Management</h2>
        <p className="text-secondary-300 mb-4">
          Control and monitor your trading strategies from this dashboard. Activate, pause, or stop strategies,
          adjust parameters, and view performance metrics.
        </p>
      </div>
      
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {filteredStrategies.length === 0 ? (
          <div className="col-span-2 card p-8 text-center text-secondary-400">
            No strategies found matching the selected filter.
          </div>
        ) : (
          filteredStrategies.map((strategy: Strategy) => (
            <StrategyCard
              key={strategy.id}
              strategy={strategy}
              onStatusChange={handleStatusChange}
            />
          ))
        )}
      </div>
    </div>
  );
};

export default Strategies;
import React, { useState } from 'react';
import { usePositionStore } from '../../store/positionStore';
import { Position } from '../../types';
import { format } from 'date-fns';
import {
  ArrowUpIcon,
  ArrowDownIcon,
  XMarkIcon,
  FunnelIcon,
  ArrowsRightLeftIcon,
} from '@heroicons/react/24/outline';
import clsx from 'clsx';

const PositionManagement: React.FC = () => {
  const { positions, closePositionById, setFilters, filters } = usePositionStore();
  const [selectedPosition, setSelectedPosition] = useState<Position | null>(null);
  
  const handleClosePosition = async (id: string) => {
    if (window.confirm('Are you sure you want to close this position?')) {
      await closePositionById(id);
    }
  };
  
  const handleFilterChange = (key: keyof typeof filters, value: any) => {
    setFilters({ [key]: value });
  };
  
  const statusOptions = [
    { value: 'open', label: 'Open' },
    { value: 'closed', label: 'Closed' },
    { value: 'liquidated', label: 'Liquidated' },
  ];
  
  const getStatusBadgeClass = (status: string) => {
    switch (status) {
      case 'open':
        return 'bg-success-100 text-success-800';
      case 'closed':
        return 'bg-secondary-100 text-secondary-800';
      case 'liquidated':
        return 'bg-danger-100 text-danger-800';
      default:
        return 'bg-secondary-100 text-secondary-800';
    }
  };
  
  const getRiskBadgeClass = (risk: string) => {
    switch (risk) {
      case 'low':
        return 'bg-success-100 text-success-800';
      case 'medium':
        return 'bg-warning-100 text-warning-800';
      case 'high':
        return 'bg-danger-100 text-danger-800';
      default:
        return 'bg-secondary-100 text-secondary-800';
    }
  };
  
  return (
    <div className="card">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center">
          <ArrowsRightLeftIcon className="w-5 h-5 text-primary-500 mr-2" />
          <h3 className="text-lg font-semibold">Position Management</h3>
        </div>
        
        <div className="flex items-center space-x-2">
          <div className="relative">
            <select
              className="input pr-8 appearance-none"
              value={filters.status || ''}
              onChange={(e) => handleFilterChange('status', e.target.value || undefined)}
              aria-label="Filter by status"
              title="Filter by status"
            >
              <option value="">All Statuses</option>
              {statusOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
            <FunnelIcon className="w-4 h-4 text-secondary-400 absolute right-2 top-1/2 transform -translate-y-1/2" />
          </div>
          
          <input
            type="text"
            placeholder="Filter by asset..."
            className="input"
            value={filters.asset || ''}
            onChange={(e) => handleFilterChange('asset', e.target.value || undefined)}
            aria-label="Filter by asset"
          />
        </div>
      </div>
      
      <div className="overflow-x-auto">
        <table className="w-full">
          <thead>
            <tr className="text-left text-secondary-400 text-sm border-b border-secondary-700">
              <th className="pb-2 font-medium">Asset</th>
              <th className="pb-2 font-medium">Strategy</th>
              <th className="pb-2 font-medium">Side</th>
              <th className="pb-2 font-medium">Size</th>
              <th className="pb-2 font-medium">Entry Price</th>
              <th className="pb-2 font-medium">Current Price</th>
              <th className="pb-2 font-medium">PnL</th>
              <th className="pb-2 font-medium">Status</th>
              <th className="pb-2 font-medium">Risk</th>
              <th className="pb-2 font-medium">Open Time</th>
              <th className="pb-2 font-medium">Actions</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-secondary-700">
            {positions.length === 0 ? (
              <tr>
                <td colSpan={11} className="py-4 text-center text-secondary-400">
                  No positions found
                </td>
              </tr>
            ) : (
              positions.map((position) => (
                <tr 
                  key={position.id} 
                  className="hover:bg-secondary-750 cursor-pointer"
                  onClick={() => setSelectedPosition(position)}
                >
                  <td className="py-3 font-medium">{position.asset}</td>
                  <td className="py-3 text-secondary-300">{position.strategy}</td>
                  <td className="py-3">
                    <span className={clsx(
                      'flex items-center',
                      position.side === 'long' ? 'text-success-500' : 'text-danger-500'
                    )}>
                      {position.side === 'long' ? (
                        <ArrowUpIcon className="w-4 h-4 mr-1" />
                      ) : (
                        <ArrowDownIcon className="w-4 h-4 mr-1" />
                      )}
                      {position.side.charAt(0).toUpperCase() + position.side.slice(1)}
                    </span>
                  </td>
                  <td className="py-3">{position.size.toFixed(4)}</td>
                  <td className="py-3">${position.entryPrice.toFixed(2)}</td>
                  <td className="py-3">${position.currentPrice.toFixed(2)}</td>
                  <td className={clsx(
                    'py-3 font-medium',
                    position.pnl >= 0 ? 'text-success-500' : 'text-danger-500'
                  )}>
                    ${position.pnl.toFixed(2)} ({position.pnlPercentage.toFixed(2)}%)
                  </td>
                  <td className="py-3">
                    <span className={clsx(
                      'px-2 py-1 rounded-full text-xs font-medium',
                      getStatusBadgeClass(position.status)
                    )}>
                      {position.status.charAt(0).toUpperCase() + position.status.slice(1)}
                    </span>
                  </td>
                  <td className="py-3">
                    <span className={clsx(
                      'px-2 py-1 rounded-full text-xs font-medium',
                      getRiskBadgeClass(position.risk)
                    )}>
                      {position.risk.charAt(0).toUpperCase() + position.risk.slice(1)}
                    </span>
                  </td>
                  <td className="py-3 text-secondary-300">
                    {format(new Date(position.openTime), 'MMM d, HH:mm')}
                  </td>
                  <td className="py-3">
                    {position.status === 'open' && (
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          handleClosePosition(position.id);
                        }}
                        className="p-1 text-secondary-400 hover:text-danger-500"
                        aria-label="Close position"
                        title="Close position"
                      >
                        <XMarkIcon className="w-5 h-5" />
                      </button>
                    )}
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
      
      {selectedPosition && (
        <div className="fixed inset-0 bg-secondary-900/80 flex items-center justify-center z-50">
          <div className="bg-secondary-800 rounded-lg shadow-xl max-w-2xl w-full p-6 border border-secondary-700">
            <div className="flex justify-between items-start mb-4">
              <h3 className="text-xl font-semibold">
                Position Details: {selectedPosition.asset}
              </h3>
              <button
                onClick={() => setSelectedPosition(null)}
                className="p-1 text-secondary-400 hover:text-white"
                aria-label="Close details"
                title="Close details"
              >
                <XMarkIcon className="w-6 h-6" />
              </button>
            </div>
            
            <div className="grid grid-cols-2 gap-4 mb-6">
              <div>
                <div className="text-sm text-secondary-400 mb-1">Strategy</div>
                <div className="font-medium">{selectedPosition.strategy}</div>
              </div>
              <div>
                <div className="text-sm text-secondary-400 mb-1">Side</div>
                <div className={clsx(
                  'font-medium flex items-center',
                  selectedPosition.side === 'long' ? 'text-success-500' : 'text-danger-500'
                )}>
                  {selectedPosition.side === 'long' ? (
                    <ArrowUpIcon className="w-4 h-4 mr-1" />
                  ) : (
                    <ArrowDownIcon className="w-4 h-4 mr-1" />
                  )}
                  {selectedPosition.side.charAt(0).toUpperCase() + selectedPosition.side.slice(1)}
                </div>
              </div>
              <div>
                <div className="text-sm text-secondary-400 mb-1">Size</div>
                <div className="font-medium">{selectedPosition.size.toFixed(6)}</div>
              </div>
              <div>
                <div className="text-sm text-secondary-400 mb-1">Entry Price</div>
                <div className="font-medium">${selectedPosition.entryPrice.toFixed(2)}</div>
              </div>
              <div>
                <div className="text-sm text-secondary-400 mb-1">Current Price</div>
                <div className="font-medium">${selectedPosition.currentPrice.toFixed(2)}</div>
              </div>
              <div>
                <div className="text-sm text-secondary-400 mb-1">PnL</div>
                <div className={clsx(
                  'font-medium',
                  selectedPosition.pnl >= 0 ? 'text-success-500' : 'text-danger-500'
                )}>
                  ${selectedPosition.pnl.toFixed(2)} ({selectedPosition.pnlPercentage.toFixed(2)}%)
                </div>
              </div>
              <div>
                <div className="text-sm text-secondary-400 mb-1">Status</div>
                <div>
                  <span className={clsx(
                    'px-2 py-1 rounded-full text-xs font-medium',
                    getStatusBadgeClass(selectedPosition.status)
                  )}>
                    {selectedPosition.status.charAt(0).toUpperCase() + selectedPosition.status.slice(1)}
                  </span>
                </div>
              </div>
              <div>
                <div className="text-sm text-secondary-400 mb-1">Risk Level</div>
                <div>
                  <span className={clsx(
                    'px-2 py-1 rounded-full text-xs font-medium',
                    getRiskBadgeClass(selectedPosition.risk)
                  )}>
                    {selectedPosition.risk.charAt(0).toUpperCase() + selectedPosition.risk.slice(1)}
                  </span>
                </div>
              </div>
              <div>
                <div className="text-sm text-secondary-400 mb-1">Open Time</div>
                <div className="font-medium">
                  {format(new Date(selectedPosition.openTime), 'PPpp')}
                </div>
              </div>
            </div>
            
            {selectedPosition.status === 'open' && (
              <div className="flex justify-end">
                <button
                  onClick={() => {
                    handleClosePosition(selectedPosition.id);
                    setSelectedPosition(null);
                  }}
                  className="btn btn-danger"
                >
                  Close Position
                </button>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
};

export default PositionManagement;
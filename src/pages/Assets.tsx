import React, { useState } from 'react';
import { useAssetStore } from '../store/assetStore';
import { Asset } from '../types';
import { format } from 'date-fns';
import {
  CurrencyDollarIcon,
  ArrowsRightLeftIcon,
  BanknotesIcon,
} from '@heroicons/react/24/outline';
import { Line } from 'react-chartjs-2';
import clsx from 'clsx';

const AssetCard: React.FC<{ asset: Asset }> = ({ asset }) => {
  return (
    <div className="card">
      <div className="flex justify-between items-start mb-4">
        <div>
          <h3 className="text-lg font-semibold flex items-center">
            <span className="mr-2">{asset.symbol}</span>
            <span className="text-sm text-secondary-400">{asset.name}</span>
          </h3>
        </div>
        <div className={clsx(
          'px-2 py-1 rounded-full text-xs font-medium',
          asset.priceChange24h >= 0 ? 'bg-success-100 text-success-800' : 'bg-danger-100 text-danger-800'
        )}>
          {asset.priceChange24h >= 0 ? '+' : ''}{asset.priceChange24h.toFixed(2)}%
        </div>
      </div>
      
      <div className="grid grid-cols-2 gap-4 mb-4">
        <div>
          <div className="text-sm text-secondary-400 mb-1">Balance</div>
          <div className="text-lg font-semibold">
            {asset.balance.toFixed(6)} {asset.symbol}
          </div>
        </div>
        <div>
          <div className="text-sm text-secondary-400 mb-1">Value (USD)</div>
          <div className="text-lg font-semibold">
            ${asset.valueUsd.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
          </div>
        </div>
        <div>
          <div className="text-sm text-secondary-400 mb-1">Price</div>
          <div className="text-lg font-semibold">
            ${asset.price.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 6 })}
          </div>
        </div>
        <div>
          <div className="text-sm text-secondary-400 mb-1">Allocation</div>
          <div className="text-lg font-semibold">
            {asset.allocation.toFixed(2)}%
          </div>
        </div>
      </div>
      
      <div className="flex justify-between">
        <button className="btn btn-primary text-sm py-1 px-3 flex items-center">
          <BanknotesIcon className="w-4 h-4 mr-1" />
          Deposit
        </button>
        <button className="btn btn-secondary text-sm py-1 px-3 flex items-center">
          <ArrowsRightLeftIcon className="w-4 h-4 mr-1" />
          Withdraw
        </button>
      </div>
    </div>
  );
};

const Assets: React.FC = () => {
  const { assets } = useAssetStore();
  const [timeframe, setTimeframe] = useState<'day' | 'week' | 'month' | 'year'>('week');
  
  // Calculate total portfolio value
  const totalValue = assets.reduce((sum, asset) => sum + asset.valueUsd, 0);
  
  // Mock data for the chart - in a real app, this would come from the API
  const balanceHistoryData = {
    labels: Array.from({ length: 24 }, (_, i) => format(new Date(Date.now() - i * 5 * 60000), 'HH:mm')).reverse(),
    datasets: [
      {
        label: 'Portfolio Value',
        data: Array.from({ length: 24 }, (_, i) => {
          const base = totalValue * 0.9;
          const variance = totalValue * 0.2 * Math.random();
          return base + variance;
        }).reverse(),
        borderColor: 'rgba(14, 165, 233, 1)',
        backgroundColor: 'rgba(14, 165, 233, 0.1)',
        tension: 0.4,
        fill: true,
      },
    ],
  };
  
  const chartOptions = {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: {
        display: false,
      },
      tooltip: {
        mode: 'index' as const,
        intersect: false,
        callbacks: {
          label: function(context: any) {
            let label = context.dataset.label || '';
            if (label) {
              label += ': ';
            }
            if (context.parsed.y !== null) {
              label += new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' }).format(context.parsed.y);
            }
            return label;
          }
        }
      },
    },
    scales: {
      x: {
        grid: {
          color: 'rgba(51, 65, 85, 0.5)',
        },
        ticks: {
          color: 'rgba(148, 163, 184, 1)',
          maxRotation: 0,
          autoSkip: true,
          maxTicksLimit: 8,
        },
      },
      y: {
        grid: {
          color: 'rgba(51, 65, 85, 0.5)',
        },
        ticks: {
          color: 'rgba(148, 163, 184, 1)',
          callback: function(value: any) {
            return new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD', notation: 'compact' }).format(value);
          }
        },
      },
    },
    interaction: {
      mode: 'nearest' as const,
      axis: 'x' as const,
      intersect: false,
    },
  };
  
  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold mb-6">Asset Dashboard</h1>
      
      <div className="card mb-6">
        <div className="flex justify-between items-start mb-6">
          <div>
            <h2 className="text-xl font-semibold flex items-center">
              <CurrencyDollarIcon className="w-6 h-6 text-primary-500 mr-2" />
              Portfolio Overview
            </h2>
            <p className="text-secondary-400 mt-1">
              Total Value: ${totalValue.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
            </p>
          </div>
          
          <div className="flex space-x-2">
            <button
              onClick={() => setTimeframe('day')}
              className={clsx(
                'px-3 py-1 rounded-md text-sm',
                timeframe === 'day'
                  ? 'bg-primary-600 text-white'
                  : 'bg-secondary-700 text-secondary-300 hover:bg-secondary-600'
              )}
            >
              Day
            </button>
            <button
              onClick={() => setTimeframe('week')}
              className={clsx(
                'px-3 py-1 rounded-md text-sm',
                timeframe === 'week'
                  ? 'bg-primary-600 text-white'
                  : 'bg-secondary-700 text-secondary-300 hover:bg-secondary-600'
              )}
            >
              Week
            </button>
            <button
              onClick={() => setTimeframe('month')}
              className={clsx(
                'px-3 py-1 rounded-md text-sm',
                timeframe === 'month'
                  ? 'bg-primary-600 text-white'
                  : 'bg-secondary-700 text-secondary-300 hover:bg-secondary-600'
              )}
            >
              Month
            </button>
            <button
              onClick={() => setTimeframe('year')}
              className={clsx(
                'px-3 py-1 rounded-md text-sm',
                timeframe === 'year'
                  ? 'bg-primary-600 text-white'
                  : 'bg-secondary-700 text-secondary-300 hover:bg-secondary-600'
              )}
            >
              Year
            </button>
          </div>
        </div>
        
        <div className="h-64">
          <Line data={balanceHistoryData} options={chartOptions} />
        </div>
      </div>
      
      <h2 className="text-xl font-semibold mb-4">Your Assets</h2>
      
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {assets.length === 0 ? (
          <div className="col-span-3 card p-8 text-center text-secondary-400">
            No assets found.
          </div>
        ) : (
          assets.map(asset => (
            <AssetCard key={asset.symbol} asset={asset} />
          ))
        )}
      </div>
    </div>
  );
};

export default Assets;
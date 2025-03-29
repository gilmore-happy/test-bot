import React from 'react';
import { useSystemStore } from '../../store/systemStore';
import { Line } from 'react-chartjs-2';
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend,
  Filler
} from 'chart.js';
import { format } from 'date-fns';
import { CpuChipIcon, ServerStackIcon, WifiIcon } from '@heroicons/react/24/outline';

// Register ChartJS components
ChartJS.register(
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend,
  Filler
);

const PerformanceMonitoring: React.FC = () => {
  const { performanceMetrics } = useSystemStore();
  
  // Mock data for the charts - in a real app, this would come from the API
  const latencyData = {
    labels: Array.from({ length: 24 }, (_, i) => format(new Date(Date.now() - i * 5 * 60000), 'HH:mm')).reverse(),
    datasets: [
      {
        label: 'Network',
        data: Array.from({ length: 24 }, () => Math.floor(Math.random() * 50 + 20)),
        borderColor: 'rgba(14, 165, 233, 1)',
        backgroundColor: 'rgba(14, 165, 233, 0.1)',
        tension: 0.4,
        fill: false,
      },
      {
        label: 'Execution',
        data: Array.from({ length: 24 }, () => Math.floor(Math.random() * 100 + 50)),
        borderColor: 'rgba(34, 197, 94, 1)',
        backgroundColor: 'rgba(34, 197, 94, 0.1)',
        tension: 0.4,
        fill: false,
      },
      {
        label: 'Confirmation',
        data: Array.from({ length: 24 }, () => Math.floor(Math.random() * 200 + 300)),
        borderColor: 'rgba(245, 158, 11, 1)',
        backgroundColor: 'rgba(245, 158, 11, 0.1)',
        tension: 0.4,
        fill: false,
      },
    ],
  };
  
  const successRateData = {
    labels: Array.from({ length: 24 }, (_, i) => format(new Date(Date.now() - i * 5 * 60000), 'HH:mm')).reverse(),
    datasets: [
      {
        label: 'Success Rate',
        data: Array.from({ length: 24 }, () => Math.floor(Math.random() * 10 + 90)),
        borderColor: 'rgba(34, 197, 94, 1)',
        backgroundColor: 'rgba(34, 197, 94, 0.2)',
        tension: 0.4,
        fill: true,
      },
    ],
  };
  
  const resourceUtilizationData = {
    labels: Array.from({ length: 24 }, (_, i) => format(new Date(Date.now() - i * 5 * 60000), 'HH:mm')).reverse(),
    datasets: [
      {
        label: 'CPU',
        data: Array.from({ length: 24 }, () => Math.floor(Math.random() * 40 + 20)),
        borderColor: 'rgba(14, 165, 233, 1)',
        backgroundColor: 'rgba(14, 165, 233, 0.1)',
        tension: 0.4,
        fill: true,
      },
      {
        label: 'Memory',
        data: Array.from({ length: 24 }, () => Math.floor(Math.random() * 30 + 40)),
        borderColor: 'rgba(245, 158, 11, 1)',
        backgroundColor: 'rgba(245, 158, 11, 0.1)',
        tension: 0.4,
        fill: true,
      },
      {
        label: 'Network',
        data: Array.from({ length: 24 }, () => Math.floor(Math.random() * 50 + 10)),
        borderColor: 'rgba(34, 197, 94, 1)',
        backgroundColor: 'rgba(34, 197, 94, 0.1)',
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
        position: 'top' as const,
        labels: {
          color: 'rgba(203, 213, 225, 1)',
        },
      },
      tooltip: {
        mode: 'index' as const,
        intersect: false,
      },
    },
    scales: {
      x: {
        grid: {
          color: 'rgba(51, 65, 85, 0.5)',
        },
        ticks: {
          color: 'rgba(148, 163, 184, 1)',
        },
      },
      y: {
        grid: {
          color: 'rgba(51, 65, 85, 0.5)',
        },
        ticks: {
          color: 'rgba(148, 163, 184, 1)',
        },
      },
    },
    interaction: {
      mode: 'nearest' as const,
      axis: 'x' as const,
      intersect: false,
    },
  };
  
  const successRateOptions = {
    ...chartOptions,
    scales: {
      ...chartOptions.scales,
      y: {
        ...chartOptions.scales.y,
        min: 80,
        max: 100,
      },
    },
  };
  
  return (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <div className="card">
        <div className="flex items-center mb-4">
          <WifiIcon className="w-5 h-5 text-primary-500 mr-2" />
          <h3 className="text-lg font-semibold">Latency Metrics</h3>
        </div>
        <div className="h-64">
          <Line data={latencyData} options={chartOptions} />
        </div>
        <div className="grid grid-cols-3 gap-4 mt-4">
          <div className="text-center">
            <div className="text-sm text-secondary-400">Network</div>
            <div className="text-xl font-semibold text-primary-500">
              {performanceMetrics?.latency.network ? `${Math.round(performanceMetrics.latency.network)}ms` : 'N/A'}
            </div>
          </div>
          <div className="text-center">
            <div className="text-sm text-secondary-400">Execution</div>
            <div className="text-xl font-semibold text-success-500">
              {performanceMetrics?.latency.execution ? `${Math.round(performanceMetrics.latency.execution)}ms` : 'N/A'}
            </div>
          </div>
          <div className="text-center">
            <div className="text-sm text-secondary-400">Confirmation</div>
            <div className="text-xl font-semibold text-warning-500">
              {performanceMetrics?.latency.confirmation ? `${Math.round(performanceMetrics.latency.confirmation)}ms` : 'N/A'}
            </div>
          </div>
        </div>
      </div>
      
      <div className="card">
        <div className="flex items-center mb-4">
          <ServerStackIcon className="w-5 h-5 text-success-500 mr-2" />
          <h3 className="text-lg font-semibold">Success Rate</h3>
        </div>
        <div className="h-64">
          <Line data={successRateData} options={successRateOptions} />
        </div>
        <div className="grid grid-cols-2 gap-4 mt-4">
          <div className="text-center">
            <div className="text-sm text-secondary-400">Success Rate</div>
            <div className="text-xl font-semibold text-success-500">
              {performanceMetrics?.successRate ? `${performanceMetrics.successRate.toFixed(2)}%` : 'N/A'}
            </div>
          </div>
          <div className="text-center">
            <div className="text-sm text-secondary-400">Failure Rate</div>
            <div className="text-xl font-semibold text-danger-500">
              {performanceMetrics?.failureRate ? `${performanceMetrics.failureRate.toFixed(2)}%` : 'N/A'}
            </div>
          </div>
        </div>
      </div>
      
      <div className="card lg:col-span-2">
        <div className="flex items-center mb-4">
          <CpuChipIcon className="w-5 h-5 text-warning-500 mr-2" />
          <h3 className="text-lg font-semibold">Resource Utilization</h3>
        </div>
        <div className="h-64">
          <Line data={resourceUtilizationData} options={chartOptions} />
        </div>
        <div className="grid grid-cols-3 gap-4 mt-4">
          <div className="text-center">
            <div className="text-sm text-secondary-400">CPU Usage</div>
            <div className="text-xl font-semibold text-primary-500">
              {performanceMetrics?.resourceUtilization.cpu ? `${performanceMetrics.resourceUtilization.cpu.toFixed(1)}%` : 'N/A'}
            </div>
          </div>
          <div className="text-center">
            <div className="text-sm text-secondary-400">Memory Usage</div>
            <div className="text-xl font-semibold text-warning-500">
              {performanceMetrics?.resourceUtilization.memory ? `${performanceMetrics.resourceUtilization.memory.toFixed(1)}%` : 'N/A'}
            </div>
          </div>
          <div className="text-center">
            <div className="text-sm text-secondary-400">Network Usage</div>
            <div className="text-xl font-semibold text-success-500">
              {performanceMetrics?.resourceUtilization.network ? `${performanceMetrics.resourceUtilization.network.toFixed(1)} Mbps` : 'N/A'}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default PerformanceMonitoring;
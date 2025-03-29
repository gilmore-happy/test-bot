import React from 'react';
import PerformanceMonitoring from '../components/dashboard/PerformanceMonitoring';

const Performance: React.FC = () => {
  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold mb-6">Performance Monitoring</h1>
      
      <div className="card mb-6">
        <h2 className="text-xl font-semibold mb-4">Performance Overview</h2>
        <p className="text-secondary-300 mb-4">
          This dashboard provides detailed metrics on system performance, including latency, success rates, and resource utilization.
          Monitor these metrics to ensure optimal trading performance and identify potential bottlenecks.
        </p>
      </div>
      
      <PerformanceMonitoring />
    </div>
  );
};

export default Performance;
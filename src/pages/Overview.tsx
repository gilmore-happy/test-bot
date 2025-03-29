import React from 'react';
import OverviewPanel from '../components/dashboard/OverviewPanel';
import PerformanceMonitoring from '../components/dashboard/PerformanceMonitoring';

const Overview: React.FC = () => {
  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold mb-6">Dashboard Overview</h1>
      
      <OverviewPanel />
      
      <PerformanceMonitoring />
    </div>
  );
};

export default Overview;
import React from 'react';
import PositionManagement from '../components/dashboard/PositionManagement';

const Positions: React.FC = () => {
  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold mb-6">Position Management</h1>
      
      <div className="card mb-6">
        <h2 className="text-xl font-semibold mb-4">Active Positions</h2>
        <p className="text-secondary-300 mb-4">
          Monitor and manage all your trading positions from this dashboard. View real-time performance, 
          risk exposure, and take action when needed.
        </p>
      </div>
      
      <PositionManagement />
    </div>
  );
};

export default Positions;
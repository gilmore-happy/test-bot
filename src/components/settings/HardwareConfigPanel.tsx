import React, { useEffect, useState } from 'react';
import { useSystemStore } from '../../store/systemStore';
import { ServerIcon, CpuChipIcon, DeviceTabletIcon, GlobeAltIcon } from '@heroicons/react/24/outline';
import { CpuConfig, MemoryConfig, NetworkInterfaceConfig } from '../../types';
import clsx from 'clsx';

const HardwareConfigPanel: React.FC = () => {
  const {
    config,
    hardwareCapabilities,
    hardwareProfiles,
    featureFlags,
    loading,
    fetchConfig,
    fetchHardwareCapabilities,
    fetchHardwareProfiles,
    fetchFeatureFlags,
    applyProfile,
    updateHardwareConfig,
    toggleFeature
  } = useSystemStore();

  const [selectedProfile, setSelectedProfile] = useState<string>('');
  const [cpuConfig, setCpuConfig] = useState<CpuConfig>({
    physical_cores: undefined,
    logical_threads: undefined,
    frequency_mhz: undefined,
    use_cpu_pinning: false,
    isolated_cores: [],
    numa_aware: false,
    numa_node: undefined
  } as CpuConfig);
  const [memoryConfig, setMemoryConfig] = useState<MemoryConfig>({
    total_mb: undefined,
    use_huge_pages: false,
    huge_page_size_kb: 2048,
    huge_pages_count: undefined,
    allocation_strategy: 'balanced'
  } as MemoryConfig);
  const [selectedInterface, setSelectedInterface] = useState<string>('');

  useEffect(() => {
    fetchConfig();
    fetchHardwareCapabilities();
    fetchHardwareProfiles();
    fetchFeatureFlags();
  }, [fetchConfig, fetchHardwareCapabilities, fetchHardwareProfiles, fetchFeatureFlags]);

  useEffect(() => {
    if (config?.hardware) {
      setCpuConfig(config.hardware.cpu);
      setMemoryConfig(config.hardware.memory);
      setSelectedProfile(config.hardware.profile || '');
    }
  }, [config]);

  const handleApplyProfile = async () => {
    if (selectedProfile) {
      await applyProfile(selectedProfile);
    }
  };

  const handleCpuConfigChange = (key: keyof CpuConfig, value: any) => {
    setCpuConfig((prev: CpuConfig) => ({ ...prev, [key]: value }));
  };

  const handleMemoryConfigChange = (key: keyof MemoryConfig, value: any) => {
    setMemoryConfig((prev: MemoryConfig) => ({ ...prev, [key]: value }));
  };

  const handleSaveCpuConfig = async () => {
    await updateHardwareConfig({ cpu: cpuConfig });
  };

  const handleSaveMemoryConfig = async () => {
    await updateHardwareConfig({ memory: memoryConfig });
  };

  const handleToggleFeature = async (category: string, feature: string, enabled: boolean) => {
    const featureName = `${category}.${feature}`;
    await toggleFeature(featureName, enabled);
  };

  const getNetworkInterface = (name: string): NetworkInterfaceConfig | undefined => {
    return config?.hardware.network_interfaces.find(iface => iface.name === name);
  };

  const handleNetworkInterfaceChange = (name: string, key: keyof NetworkInterfaceConfig, value: any) => {
    const interfaces = [...(config?.hardware.network_interfaces || [])];
    const index = interfaces.findIndex(iface => iface.name === name);
    
    if (index !== -1) {
      interfaces[index] = { ...interfaces[index], [key]: value };
      updateHardwareConfig({ network_interfaces: interfaces });
    }
  };

  if (!config || !hardwareCapabilities) {
    return (
      <div className="card p-6">
        <div className="flex items-center mb-4">
          <ServerIcon className="w-6 h-6 text-primary-500 mr-2" />
          <h2 className="text-xl font-semibold">Hardware Configuration</h2>
        </div>
        <div className="flex justify-center items-center h-40">
          <div className="animate-pulse text-secondary-400">
            {loading ? 'Loading hardware configuration...' : 'Hardware configuration not available'}
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="card p-6">
        <div className="flex items-center mb-4">
          <ServerIcon className="w-6 h-6 text-primary-500 mr-2" />
          <h2 className="text-xl font-semibold">Hardware Configuration</h2>
        </div>

        <div className="mb-6">
          <h3 className="text-lg font-medium mb-2">Hardware Profiles</h3>
          <p className="text-secondary-400 text-sm mb-4">
            Select a predefined hardware profile or customize individual settings below
          </p>
          
          <div className="flex items-center space-x-4">
            <select
              value={selectedProfile}
              onChange={(e) => setSelectedProfile(e.target.value)}
              className="input flex-grow"
              disabled={loading}
              aria-label="Select hardware profile"
              title="Select hardware profile"
            >
              <option value="">Select a profile</option>
              {hardwareProfiles.map(profile => (
                <option key={profile} value={profile}>{profile}</option>
              ))}
            </select>
            <button
              onClick={handleApplyProfile}
              disabled={!selectedProfile || loading}
              className="btn btn-primary"
            >
              Apply Profile
            </button>
          </div>
        </div>

        <div className="border-t border-secondary-700 pt-6 mb-6">
          <div className="flex items-center mb-4">
            <CpuChipIcon className="w-5 h-5 text-primary-500 mr-2" />
            <h3 className="text-lg font-medium">CPU Configuration</h3>
          </div>
          
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
            <div>
              <label className="block text-sm font-medium mb-1">Physical Cores</label>
              <input
                type="number"
                value={cpuConfig.physical_cores || ''}
                onChange={(e) => handleCpuConfigChange('physical_cores', parseInt(e.target.value) || undefined)}
                className="input w-full"
                placeholder={`Detected: ${hardwareCapabilities.cpu.physical_cores}`}
                min={1}
                max={hardwareCapabilities.cpu.physical_cores}
              />
            </div>
            
            <div>
              <label className="block text-sm font-medium mb-1">Logical Threads</label>
              <input
                type="number"
                value={cpuConfig.logical_threads || ''}
                onChange={(e) => handleCpuConfigChange('logical_threads', parseInt(e.target.value) || undefined)}
                className="input w-full"
                placeholder={`Detected: ${hardwareCapabilities.cpu.logical_threads}`}
                min={1}
                max={hardwareCapabilities.cpu.logical_threads}
              />
            </div>
          </div>
          
          <div className="space-y-3 mb-4">
            <div className="flex items-center justify-between">
              <div>
                <span className="font-medium">CPU Pinning</span>
                <p className="text-sm text-secondary-400">Pin processes to specific CPU cores</p>
              </div>
              <div className="relative inline-block w-12 h-6 transition duration-200 ease-in-out rounded-full">
                <input
                  type="checkbox"
                  className="absolute w-6 h-6 opacity-0 z-10 cursor-pointer"
                  checked={cpuConfig.use_cpu_pinning}
                  onChange={(e) => handleCpuConfigChange('use_cpu_pinning', e.target.checked)}
                  disabled={!featureFlags?.cpu.cpu_pinning}
                  aria-label="Enable CPU pinning"
                  title="Enable CPU pinning"
                />
                <div className={clsx(
                  "w-12 h-6 transition-colors duration-200 ease-in-out rounded-full",
                  cpuConfig.use_cpu_pinning ? "bg-primary-600" : "bg-secondary-600",
                  !featureFlags?.cpu.cpu_pinning && "opacity-50"
                )}></div>
                <div className={clsx(
                  "absolute top-0.5 left-0.5 bg-white w-5 h-5 transition-transform duration-200 ease-in-out rounded-full",
                  cpuConfig.use_cpu_pinning ? "transform translate-x-6" : ""
                )}></div>
              </div>
            </div>
            
            <div className="flex items-center justify-between">
              <div>
                <span className="font-medium">NUMA Awareness</span>
                <p className="text-sm text-secondary-400">Optimize for NUMA architecture</p>
              </div>
              <div className="relative inline-block w-12 h-6 transition duration-200 ease-in-out rounded-full">
                <input
                  type="checkbox"
                  className="absolute w-6 h-6 opacity-0 z-10 cursor-pointer"
                  checked={cpuConfig.numa_aware}
                  onChange={(e) => handleCpuConfigChange('numa_aware', e.target.checked)}
                  disabled={!featureFlags?.capabilities.numa_awareness}
                  aria-label="Enable NUMA awareness"
                  title="Enable NUMA awareness"
                />
                <div className={clsx(
                  "w-12 h-6 transition-colors duration-200 ease-in-out rounded-full",
                  cpuConfig.numa_aware ? "bg-primary-600" : "bg-secondary-600",
                  !featureFlags?.capabilities.numa_awareness && "opacity-50"
                )}></div>
                <div className={clsx(
                  "absolute top-0.5 left-0.5 bg-white w-5 h-5 transition-transform duration-200 ease-in-out rounded-full",
                  cpuConfig.numa_aware ? "transform translate-x-6" : ""
                )}></div>
              </div>
            </div>
          </div>
          
          <div className="flex justify-end">
            <button
              onClick={handleSaveCpuConfig}
              disabled={loading}
              className="btn btn-primary"
            >
              Save CPU Settings
            </button>
          </div>
        </div>
        
        <div className="border-t border-secondary-700 pt-6 mb-6">
          <div className="flex items-center mb-4">
            <DeviceTabletIcon className="w-5 h-5 text-primary-500 mr-2" />
            <h3 className="text-lg font-medium">Memory Configuration</h3>
          </div>
          
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
            <div>
              <label className="block text-sm font-medium mb-1">Total Memory (MB)</label>
              <input
                type="number"
                value={memoryConfig.total_mb || ''}
                onChange={(e) => handleMemoryConfigChange('total_mb', parseInt(e.target.value) || undefined)}
                className="input w-full"
                placeholder={`Detected: ${hardwareCapabilities.memory.total_mb}`}
                min={1024}
                max={hardwareCapabilities.memory.total_mb}
              />
            </div>
            
            <div>
              <label className="block text-sm font-medium mb-1">Memory Allocation Strategy</label>
              <select
                value={memoryConfig.allocation_strategy}
                onChange={(e) => handleMemoryConfigChange('allocation_strategy', e.target.value)}
                className="input w-full"
                aria-label="Memory allocation strategy"
                title="Memory allocation strategy"
              >
                <option value="conservative">Conservative</option>
                <option value="balanced">Balanced</option>
                <option value="aggressive">Aggressive</option>
              </select>
            </div>
          </div>
          
          <div className="space-y-3 mb-4">
            <div className="flex items-center justify-between">
              <div>
                <span className="font-medium">Huge Pages</span>
                <p className="text-sm text-secondary-400">Use huge pages for memory allocation</p>
              </div>
              <div className="relative inline-block w-12 h-6 transition duration-200 ease-in-out rounded-full">
                <input
                  type="checkbox"
                  className="absolute w-6 h-6 opacity-0 z-10 cursor-pointer"
                  checked={memoryConfig.use_huge_pages}
                  onChange={(e) => handleMemoryConfigChange('use_huge_pages', e.target.checked)}
                  disabled={!featureFlags?.memory.huge_pages}
                  aria-label="Enable huge pages"
                  title="Enable huge pages"
                />
                <div className={clsx(
                  "w-12 h-6 transition-colors duration-200 ease-in-out rounded-full",
                  memoryConfig.use_huge_pages ? "bg-primary-600" : "bg-secondary-600",
                  !featureFlags?.memory.huge_pages && "opacity-50"
                )}></div>
                <div className={clsx(
                  "absolute top-0.5 left-0.5 bg-white w-5 h-5 transition-transform duration-200 ease-in-out rounded-full",
                  memoryConfig.use_huge_pages ? "transform translate-x-6" : ""
                )}></div>
              </div>
            </div>
            
            {memoryConfig.use_huge_pages && (
              <div className="ml-6 space-y-3">
                <div>
                  <label className="block text-sm font-medium mb-1">Huge Page Size (KB)</label>
                  <select
                    value={memoryConfig.huge_page_size_kb}
                    onChange={(e) => handleMemoryConfigChange('huge_page_size_kb', parseInt(e.target.value))}
                    className="input w-full"
                    disabled={!featureFlags?.memory.huge_pages}
                    aria-label="Huge page size"
                    title="Huge page size"
                  >
                    <option value={2048}>2MB (2048 KB)</option>
                    {featureFlags?.memory.huge_pages_1gb && (
                      <option value={1048576}>1GB (1048576 KB)</option>
                    )}
                  </select>
                </div>
                
                <div>
                  <label className="block text-sm font-medium mb-1">Huge Pages Count</label>
                  <input
                    type="number"
                    value={memoryConfig.huge_pages_count || ''}
                    onChange={(e) => handleMemoryConfigChange('huge_pages_count', parseInt(e.target.value) || undefined)}
                    className="input w-full"
                    placeholder="Auto"
                    min={1}
                  />
                </div>
              </div>
            )}
          </div>
          
          <div className="flex justify-end">
            <button
              onClick={handleSaveMemoryConfig}
              disabled={loading}
              className="btn btn-primary"
            >
              Save Memory Settings
            </button>
          </div>
        </div>
        
        <div className="border-t border-secondary-700 pt-6">
          <div className="flex items-center mb-4">
            <GlobeAltIcon className="w-5 h-5 text-primary-500 mr-2" />
            <h3 className="text-lg font-medium">Network Interfaces</h3>
          </div>
          
          <div className="mb-4">
            <label className="block text-sm font-medium mb-1">Select Interface</label>
            <select
              value={selectedInterface}
              onChange={(e) => setSelectedInterface(e.target.value)}
              className="input w-full"
              aria-label="Select network interface"
              title="Select network interface"
            >
              <option value="">Select a network interface</option>
              {hardwareCapabilities.network_interfaces.map(iface => (
                <option key={iface.name} value={iface.name}>{iface.name} ({iface.ip_address || 'No IP'})</option>
              ))}
            </select>
          </div>
          
          {selectedInterface && (
            <div className="space-y-4">
              {(() => {
                const iface = getNetworkInterface(selectedInterface);
                const detectedIface = hardwareCapabilities.network_interfaces.find(i => i.name === selectedInterface);
                
                if (!iface || !detectedIface) return null;
                
                return (
                  <>
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                      <div>
                        <label className="block text-sm font-medium mb-1">Interface Name</label>
                        <input
                          type="text"
                          value={iface.name}
                          disabled
                          className="input w-full bg-secondary-700"
                          aria-label="Interface name"
                          title="Interface name"
                        />
                      </div>
                      
                      <div>
                        <label className="block text-sm font-medium mb-1">MAC Address</label>
                        <input
                          type="text"
                          value={iface.mac_address || detectedIface.mac_address}
                          disabled
                          className="input w-full bg-secondary-700"
                          aria-label="MAC address"
                          title="MAC address"
                        />
                      </div>
                    </div>
                    
                    <div className="space-y-3">
                      <div className="flex items-center justify-between">
                        <div>
                          <span className="font-medium">Enable Interface</span>
                          <p className="text-sm text-secondary-400">Use this interface for trading</p>
                        </div>
                        <div className="relative inline-block w-12 h-6 transition duration-200 ease-in-out rounded-full">
                          <input
                            type="checkbox"
                            className="absolute w-6 h-6 opacity-0 z-10 cursor-pointer"
                            checked={iface.enabled}
                            onChange={(e) => handleNetworkInterfaceChange(iface.name, 'enabled', e.target.checked)}
                            aria-label="Enable network interface"
                            title="Enable network interface"
                          />
                          <div className={clsx(
                            "w-12 h-6 transition-colors duration-200 ease-in-out rounded-full",
                            iface.enabled ? "bg-primary-600" : "bg-secondary-600"
                          )}></div>
                          <div className={clsx(
                            "absolute top-0.5 left-0.5 bg-white w-5 h-5 transition-transform duration-200 ease-in-out rounded-full",
                            iface.enabled ? "transform translate-x-6" : ""
                          )}></div>
                        </div>
                      </div>
                      
                      <div className="flex items-center justify-between">
                        <div>
                          <span className="font-medium">Hardware Timestamping</span>
                          <p className="text-sm text-secondary-400">Use hardware timestamping for accurate timing</p>
                        </div>
                        <div className="relative inline-block w-12 h-6 transition duration-200 ease-in-out rounded-full">
                          <input
                            type="checkbox"
                            className="absolute w-6 h-6 opacity-0 z-10 cursor-pointer"
                            checked={iface.hardware_timestamping}
                            onChange={(e) => handleNetworkInterfaceChange(iface.name, 'hardware_timestamping', e.target.checked)}
                            disabled={!detectedIface.hardware_timestamping}
                            aria-label="Enable hardware timestamping"
                            title="Enable hardware timestamping"
                          />
                          <div className={clsx(
                            "w-12 h-6 transition-colors duration-200 ease-in-out rounded-full",
                            iface.hardware_timestamping ? "bg-primary-600" : "bg-secondary-600",
                            !detectedIface.hardware_timestamping && "opacity-50"
                          )}></div>
                          <div className={clsx(
                            "absolute top-0.5 left-0.5 bg-white w-5 h-5 transition-transform duration-200 ease-in-out rounded-full",
                            iface.hardware_timestamping ? "transform translate-x-6" : ""
                          )}></div>
                        </div>
                      </div>
                      
                      <div className="flex items-center justify-between">
                        <div>
                          <span className="font-medium">TCP/UDP Offload</span>
                          <p className="text-sm text-secondary-400">Use hardware offloading for TCP/UDP</p>
                        </div>
                        <div className="relative inline-block w-12 h-6 transition duration-200 ease-in-out rounded-full">
                          <input
                            type="checkbox"
                            className="absolute w-6 h-6 opacity-0 z-10 cursor-pointer"
                            checked={iface.tcp_udp_offload}
                            onChange={(e) => handleNetworkInterfaceChange(iface.name, 'tcp_udp_offload', e.target.checked)}
                            disabled={!detectedIface.tcp_udp_offload}
                            aria-label="Enable TCP/UDP offload"
                            title="Enable TCP/UDP offload"
                          />
                          <div className={clsx(
                            "w-12 h-6 transition-colors duration-200 ease-in-out rounded-full",
                            iface.tcp_udp_offload ? "bg-primary-600" : "bg-secondary-600",
                            !detectedIface.tcp_udp_offload && "opacity-50"
                          )}></div>
                          <div className={clsx(
                            "absolute top-0.5 left-0.5 bg-white w-5 h-5 transition-transform duration-200 ease-in-out rounded-full",
                            iface.tcp_udp_offload ? "transform translate-x-6" : ""
                          )}></div>
                        </div>
                      </div>
                    </div>
                  </>
                );
              })()}
            </div>
          )}
        </div>
      </div>
      
      <div className="card p-6">
        <div className="flex items-center mb-4">
          <ServerIcon className="w-6 h-6 text-primary-500 mr-2" />
          <h2 className="text-xl font-semibold">Feature Flags</h2>
        </div>
        
        {featureFlags && (
          <div className="space-y-6">
            <div>
              <h3 className="text-lg font-medium mb-2">CPU Features</h3>
              <div className="space-y-2">
                {Object.entries(featureFlags.cpu).map(([key, enabled]) => (
                  <div key={key} className="flex items-center justify-between">
                    <span className="capitalize">{key.replace(/_/g, ' ')}</span>
                    <div className="relative inline-block w-12 h-6 transition duration-200 ease-in-out rounded-full">
                      <input
                        type="checkbox"
                        className="absolute w-6 h-6 opacity-0 z-10 cursor-pointer"
                        checked={enabled}
                        onChange={(e) => handleToggleFeature('cpu', key, e.target.checked)}
                        aria-label={`Toggle ${key.replace(/_/g, ' ')} feature`}
                        title={`Toggle ${key.replace(/_/g, ' ')} feature`}
                      />
                      <div className={clsx(
                        "w-12 h-6 transition-colors duration-200 ease-in-out rounded-full",
                        enabled ? "bg-primary-600" : "bg-secondary-600"
                      )}></div>
                      <div className={clsx(
                        "absolute top-0.5 left-0.5 bg-white w-5 h-5 transition-transform duration-200 ease-in-out rounded-full",
                        enabled ? "transform translate-x-6" : ""
                      )}></div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
            
            <div>
              <h3 className="text-lg font-medium mb-2">Memory Features</h3>
              <div className="space-y-2">
                {Object.entries(featureFlags.memory).map(([key, enabled]) => (
                  <div key={key} className="flex items-center justify-between">
                    <span className="capitalize">{key.replace(/_/g, ' ')}</span>
                    <div className="relative inline-block w-12 h-6 transition duration-200 ease-in-out rounded-full">
                      <input
                        type="checkbox"
                        className="absolute w-6 h-6 opacity-0 z-10 cursor-pointer"
                        checked={enabled}
                        onChange={(e) => handleToggleFeature('memory', key, e.target.checked)}
                        aria-label={`Toggle ${key.replace(/_/g, ' ')} feature`}
                        title={`Toggle ${key.replace(/_/g, ' ')} feature`}
                      />
                      <div className={clsx(
                        "w-12 h-6 transition-colors duration-200 ease-in-out rounded-full",
                        enabled ? "bg-primary-600" : "bg-secondary-600"
                      )}></div>
                      <div className={clsx(
                        "absolute top-0.5 left-0.5 bg-white w-5 h-5 transition-transform duration-200 ease-in-out rounded-full",
                        enabled ? "transform translate-x-6" : ""
                      )}></div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
            
            <div>
              <h3 className="text-lg font-medium mb-2">Networking Features</h3>
              <div className="space-y-2">
                {Object.entries(featureFlags.networking).map(([key, enabled]) => (
                  <div key={key} className="flex items-center justify-between">
                    <span className="capitalize">{key.replace(/_/g, ' ')}</span>
                    <div className="relative inline-block w-12 h-6 transition duration-200 ease-in-out rounded-full">
                      <input
                        type="checkbox"
                        className="absolute w-6 h-6 opacity-0 z-10 cursor-pointer"
                        checked={enabled}
                        onChange={(e) => handleToggleFeature('networking', key, e.target.checked)}
                        aria-label={`Toggle ${key.replace(/_/g, ' ')} feature`}
                        title={`Toggle ${key.replace(/_/g, ' ')} feature`}
                      />
                      <div className={clsx(
                        "w-12 h-6 transition-colors duration-200 ease-in-out rounded-full",
                        enabled ? "bg-primary-600" : "bg-secondary-600"
                      )}></div>
                      <div className={clsx(
                        "absolute top-0.5 left-0.5 bg-white w-5 h-5 transition-transform duration-200 ease-in-out rounded-full",
                        enabled ? "transform translate-x-6" : ""
                      )}></div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
            
            <div>
              <h3 className="text-lg font-medium mb-2">Additional Capabilities</h3>
              <div className="space-y-2">
                {Object.entries(featureFlags.capabilities).map(([key, enabled]) => (
                  <div key={key} className="flex items-center justify-between">
                    <span className="capitalize">{key.replace(/_/g, ' ')}</span>
                    <div className="relative inline-block w-12 h-6 transition duration-200 ease-in-out rounded-full">
                      <input
                        type="checkbox"
                        className="absolute w-6 h-6 opacity-0 z-10 cursor-pointer"
                        checked={enabled}
                        onChange={(e) => handleToggleFeature('capabilities', key, e.target.checked)}
                        aria-label={`Toggle ${key.replace(/_/g, ' ')} feature`}
                        title={`Toggle ${key.replace(/_/g, ' ')} feature`}
                      />
                      <div className={clsx(
                        "w-12 h-6 transition-colors duration-200 ease-in-out rounded-full",
                        enabled ? "bg-primary-600" : "bg-secondary-600"
                      )}></div>
                      <div className={clsx(
                        "absolute top-0.5 left-0.5 bg-white w-5 h-5 transition-transform duration-200 ease-in-out rounded-full",
                        enabled ? "transform translate-x-6" : ""
                      )}></div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default HardwareConfigPanel;
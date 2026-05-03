import React, { Suspense, useState, useEffect } from 'react';
import { useLocation } from 'react-router-dom';
import {
  Tabs,
  Box,
  Divider,
  Text,
  Loader,
  Paper,
  Title,
  RemovableScroll,
  ScrollArea,
} from '@mantine/core';
import {
  IconSettings,
  IconVideo,
  IconDeviceTv,
  IconCpu,
  IconNetwork,
  IconUser,
  IconDatabase,
  IconTerminal2,
  IconAdjustmentsHorizontal,
  IconTool,
} from '@tabler/icons-react';

const UserAgentsTable = React.lazy(
  () => import('../components/tables/UserAgentsTable.jsx')
);
const StreamProfilesTable = React.lazy(
  () => import('../components/tables/StreamProfilesTable.jsx')
);
import useAuthStore from '../store/auth';
import { USER_LEVELS } from '../constants';
import UiSettingsForm from '../components/forms/settings/UiSettingsForm.jsx';
import ErrorBoundary from '../components/ErrorBoundary.jsx';

const UserLimitsForm = React.lazy(
  () => import('../components/forms/settings/UserLimitsForm.jsx')
);
const NetworkAccessForm = React.lazy(
  () => import('../components/forms/settings/NetworkAccessForm.jsx')
);
const ProxySettingsForm = React.lazy(
  () => import('../components/forms/settings/ProxySettingsForm.jsx')
);
const StreamSettingsForm = React.lazy(
  () => import('../components/forms/settings/StreamSettingsForm.jsx')
);
const DvrSettingsForm = React.lazy(
  () => import('../components/forms/settings/DvrSettingsForm.jsx')
);
const SystemSettingsForm = React.lazy(
  () => import('../components/forms/settings/SystemSettingsForm.jsx')
);
const NavOrderForm = React.lazy(
  () => import('../components/forms/settings/NavOrderForm.jsx')
);
const MaintenanceSettingsForm = React.lazy(
  () => import('../components/forms/settings/MaintenanceSettingsForm.jsx')
);

const SettingsPage = () => {
  const authUser = useAuthStore((s) => s.user);
  const location = useLocation();

  const [activeTab, setActiveTab] = useState('ui-settings');

  // Handle hash navigation to open specific tab
  useEffect(() => {
    const hash = location.hash.replace('#', '');
    if (hash) {
      setActiveTab(hash);
    }
  }, [location.hash]);

  const isAdmin = authUser.user_level >= USER_LEVELS.ADMIN;

  return (
    <Box p="md" style={{ height: 'calc(100vh - 40px)', overflow: 'hidden' }}>
      <Title order={2} mb="xl" style={{ color: 'white' }}>
        Settings
      </Title>

      <Paper
        radius="md"
        withBorder
        p={0}
        style={{
          display: 'flex',
          height: 'calc(100% - 60px)',
          backgroundColor: 'rgba(24, 24, 27, 0.7)',
          backdropFilter: 'blur(10px)',
          borderColor: 'rgba(255, 255, 255, 0.1)',
          overflow: 'hidden',
        }}
      >
        <Tabs
          variant="unstyled"
          orientation="vertical"
          value={activeTab}
          onChange={setActiveTab}
          style={{ width: '100%', display: 'flex' }}
          styles={(theme) => ({
            root: {
              flex: 1,
            },
            tab: {
              padding: '12px 20px',
              color: 'rgba(255, 255, 255, 0.6)',
              fontWeight: 500,
              borderLeft: '3px solid transparent',
              '&[data-active]': {
                color: 'white',
                backgroundColor: 'rgba(255, 255, 255, 0.05)',
                borderLeftColor: theme.colors.blue[6],
              },
              '&:hover': {
                backgroundColor: 'rgba(255, 255, 255, 0.02)',
              },
            },
            list: {
              width: 250,
              borderRight: '1px solid rgba(255, 255, 255, 0.1)',
              padding: '10px 0',
            },
            panel: {
              flex: 1,
              padding: '30px',
              overflowY: 'auto',
              backgroundColor: 'rgba(0, 0, 0, 0.2)',
            },
          })}
        >
          <Tabs.List>
            <Tabs.Tab value="ui-settings" leftSection={<IconAdjustmentsHorizontal size={18} />}>
              General & UI
            </Tabs.Tab>

            {isAdmin && (
              <>
                <Divider my="sm" label="Backend" labelPosition="center" styles={{ label: { fontSize: 10, opacity: 0.5 } }} />
                <Tabs.Tab value="stream-settings" leftSection={<IconVideo size={18} />}>
                  Stream Sources
                </Tabs.Tab>
                <Tabs.Tab value="proxy-settings" leftSection={<IconNetwork size={18} />}>
                  Proxy Server
                </Tabs.Tab>
                <Tabs.Tab value="maintenance-settings" leftSection={<IconTool size={18} />}>
                  Maintenance
                </Tabs.Tab>
                <Tabs.Tab value="dvr-settings" leftSection={<IconDeviceTv size={18} />}>
                  DVR
                </Tabs.Tab>

                <Divider my="sm" label="System" labelPosition="center" styles={{ label: { fontSize: 10, opacity: 0.5 } }} />
                <Tabs.Tab value="system-settings" leftSection={<IconDatabase size={18} />}>
                  Data & Logs
                </Tabs.Tab>
                <Tabs.Tab value="user-agents" leftSection={<IconTerminal2 size={18} />}>
                  User-Agents
                </Tabs.Tab>
                <Tabs.Tab value="stream-profiles" leftSection={<IconCpu size={18} />}>
                  Hardware Profiles
                </Tabs.Tab>
                <Tabs.Tab value="network-access" leftSection={<IconNetwork size={18} />}>
                  Access Control
                </Tabs.Tab>
                <Tabs.Tab value="user-limits" leftSection={<IconUser size={18} />}>
                  User Limits
                </Tabs.Tab>
              </>
            )}
          </Tabs.List>

          <Tabs.Panel value="ui-settings">
            <Title order={3} mb="lg">General & UI Settings</Title>
            <UiSettingsForm active={activeTab === 'ui-settings'} />
            <Divider my="xl" label="Navigation Order" />
            <ErrorBoundary>
              <Suspense fallback={<Loader />}>
                <NavOrderForm active={activeTab === 'ui-settings'} />
              </Suspense>
            </ErrorBoundary>
          </Tabs.Panel>

          {isAdmin && (
            <>
              <Tabs.Panel value="stream-settings">
                <Title order={3} mb="lg">Stream Source Settings</Title>
                <ErrorBoundary>
                  <Suspense fallback={<Loader />}>
                    <StreamSettingsForm active={activeTab === 'stream-settings'} />
                  </Suspense>
                </ErrorBoundary>
              </Tabs.Panel>

              <Tabs.Panel value="proxy-settings">
                <Title order={3} mb="lg">Proxy Server Configuration</Title>
                <ErrorBoundary>
                  <Suspense fallback={<Loader />}>
                    <ProxySettingsForm active={activeTab === 'proxy-settings'} />
                  </Suspense>
                </ErrorBoundary>
              </Tabs.Panel>

              <Tabs.Panel value="maintenance-settings">
                <Title order={3} mb="lg">Background Maintenance</Title>
                <Text size="sm" color="dimmed" mb="xl">
                  Configure automated health checks, off-hours schedules, and idle-time triggers.
                </Text>
                <ErrorBoundary>
                  <Suspense fallback={<Loader />}>
                    <MaintenanceSettingsForm active={activeTab === 'maintenance-settings'} />
                  </Suspense>
                </ErrorBoundary>
              </Tabs.Panel>

              <Tabs.Panel value="dvr-settings">
                <Title order={3} mb="lg">DVR Settings</Title>
                <ErrorBoundary>
                  <Suspense fallback={<Loader />}>
                    <DvrSettingsForm active={activeTab === 'dvr-settings'} />
                  </Suspense>
                </ErrorBoundary>
              </Tabs.Panel>

              <Tabs.Panel value="system-settings">
                <Title order={3} mb="lg">System Data & Logging</Title>
                <ErrorBoundary>
                  <Suspense fallback={<Loader />}>
                    <SystemSettingsForm active={activeTab === 'system-settings'} />
                  </Suspense>
                </ErrorBoundary>
              </Tabs.Panel>

              <Tabs.Panel value="user-agents">
                <Title order={3} mb="lg">Custom User-Agents</Title>
                <ErrorBoundary>
                  <Suspense fallback={<Loader />}>
                    <UserAgentsTable active={activeTab === 'user-agents'} />
                  </Suspense>
                </ErrorBoundary>
              </Tabs.Panel>

              <Tabs.Panel value="stream-profiles">
                <Title order={3} mb="lg">Stream Profiles & Hardware Acceleration</Title>
                <ErrorBoundary>
                  <Suspense fallback={<Loader />}>
                    <StreamProfilesTable active={activeTab === 'stream-profiles'} />
                  </Suspense>
                </ErrorBoundary>
              </Tabs.Panel>

              <Tabs.Panel value="network-access">
                <Title order={3} mb="lg">Network Access Control (CIDR)</Title>
                <ErrorBoundary>
                  <Suspense fallback={<Loader />}>
                    <NetworkAccessForm active={activeTab === 'network-access'} />
                  </Suspense>
                </ErrorBoundary>
              </Tabs.Panel>

              <Tabs.Panel value="user-limits">
                <Title order={3} mb="lg">Concurrent User Limits</Title>
                <ErrorBoundary>
                  <Suspense fallback={<Loader />}>
                    <UserLimitsForm active={activeTab === 'user-limits'} />
                  </Suspense>
                </ErrorBoundary>
              </Tabs.Panel>
            </>
          )}
        </Tabs>
      </Paper>
    </Box>
  );
};

export default SettingsPage;

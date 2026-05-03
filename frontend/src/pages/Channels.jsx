import React, { useCallback, useRef, useState } from 'react';
import ChannelsTable from '../components/tables/ChannelsTable';
import ChannelManager from '../components/ChannelManager/index.jsx';
import { Box, Tabs, rem } from '@mantine/core';
import { LayoutGrid, Settings2 } from 'lucide-react';
import { USER_LEVELS } from '../constants';
import useAuthStore from '../store/auth';
import useLogosStore from '../store/logos';
import ErrorBoundary from '../components/ErrorBoundary';

const PageContent = () => {
  const authUser = useAuthStore((s) => s.user);
  const fetchChannelAssignableLogos = useLogosStore(
    (s) => s.fetchChannelAssignableLogos
  );
  const enableLogoRendering = useLogosStore((s) => s.enableLogoRendering);

  const channelsReady = useRef(false);
  const managerReady = useRef(false);
  const logosTriggered = useRef(false);

  const [activeTab, setActiveTab] = useState('channels');

  // Only load logos when channels table is ready
  const tryLoadLogos = useCallback(() => {
    if (channelsReady.current && !logosTriggered.current) {
      logosTriggered.current = true;
      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          enableLogoRendering();
          fetchChannelAssignableLogos();
        });
      });
    }
  }, [fetchChannelAssignableLogos, enableLogoRendering]);

  const handleChannelsReady = useCallback(() => {
    channelsReady.current = true;
    tryLoadLogos();
  }, [tryLoadLogos]);

  if (!authUser.id) return <></>;

  if (authUser.user_level <= USER_LEVELS.STANDARD) {
    return (
      <Box style={{ padding: 10 }}>
        <ChannelsTable onReady={handleChannelsReady} />
      </Box>
    );
  }

  const iconStyle = { width: rem(14), height: rem(14) };

  return (
    <Box h={'100vh'} w={'100%'} display={'flex'} style={{ flexDirection: 'column', overflowX: 'auto' }}>
      <Tabs value={activeTab} onChange={setActiveTab} style={{ flexGrow: 1, display: 'flex', flexDirection: 'column' }}>
        <Tabs.List px="md" pt="sm">
          <Tabs.Tab value="channels" leftSection={<LayoutGrid style={iconStyle} />}>
            Channels Grid
          </Tabs.Tab>
          <Tabs.Tab value="manager" leftSection={<Settings2 style={iconStyle} />}>
            Channel Manager
          </Tabs.Tab>
        </Tabs.List>

        <Tabs.Panel value="channels" style={{ flexGrow: 1, padding: 10, minHeight: 0 }}>
          <Box h="100%">
            <ChannelsTable onReady={handleChannelsReady} />
          </Box>
        </Tabs.Panel>

        <Tabs.Panel value="manager" style={{ flexGrow: 1, padding: 10, minHeight: 0 }}>
          <Box h="100%">
            {activeTab === 'manager' && <ChannelManager />}
          </Box>
        </Tabs.Panel>
      </Tabs>
    </Box>
  );
};

const ChannelsPage = () => {
  return (
    <ErrorBoundary>
      <PageContent />
    </ErrorBoundary>
  );
};

export default ChannelsPage;

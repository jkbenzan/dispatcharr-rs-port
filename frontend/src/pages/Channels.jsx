import React, { useCallback, useRef } from 'react';
import ChannelsTable from '../components/tables/ChannelsTable';
import { Box } from '@mantine/core';
import { USER_LEVELS } from '../constants';
import useAuthStore from '../store/auth';
import useLogosStore from '../store/logos';
import ErrorBoundary from '../components/ErrorBoundary';

/**
 * Channels page — displays the ChannelsTable grid.
 *
 * The Channel Manager (Angular mini-app) has been detached into its own
 * top-level route at /channel-manager. This page now only shows the
 * Channels Grid, which will eventually be replaced by the Angular
 * Channel Manager entirely.
 */
const PageContent = () => {
  const authUser = useAuthStore((s) => s.user);
  const fetchChannelAssignableLogos = useLogosStore(
    (s) => s.fetchChannelAssignableLogos
  );
  const enableLogoRendering = useLogosStore((s) => s.enableLogoRendering);

  const channelsReady = useRef(false);
  const logosTriggered = useRef(false);

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

  return (
    <Box h={'100vh'} w={'100%'} display={'flex'} style={{ flexDirection: 'column', overflowX: 'auto' }}>
      <Box style={{ flexGrow: 1, padding: 10, minHeight: 0 }}>
        <Box h="100%">
          <ChannelsTable onReady={handleChannelsReady} />
        </Box>
      </Box>
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

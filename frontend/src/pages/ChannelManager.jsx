import React from 'react';
import { Box } from '@mantine/core';
import ErrorBoundary from '../components/ErrorBoundary';

/**
 * Standalone Channel Manager page — renders the Angular mini-app
 * in a full-height iframe. This was previously a sub-tab of the
 * Channels page; it is now its own top-level route (/channel-manager)
 * to give it first-class navigation status.
 */
const PageContent = () => {
  return (
    <Box h="100vh" w="100%" display="flex" style={{ flexDirection: 'column' }}>
      <iframe
        src="/channel-manager/index.html"
        style={{ width: '100%', height: '100%', border: 'none' }}
        title="Angular Channel Manager"
      />
    </Box>
  );
};

const ChannelManagerPage = () => {
  return (
    <ErrorBoundary>
      <PageContent />
    </ErrorBoundary>
  );
};

export default ChannelManagerPage;

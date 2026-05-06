import React from 'react';
import { Box } from '@mantine/core';
import ErrorBoundary from '../components/ErrorBoundary';

/**
 * Standalone Channel Manager page — renders the Angular mini-app
 * in a full-height iframe. Previously a sub-tab of the Channels page;
 * now its own top-level route (/channel-manager).
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

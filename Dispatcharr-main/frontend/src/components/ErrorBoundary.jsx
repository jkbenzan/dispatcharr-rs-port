import React from 'react';
import { Box, Button, Center, Paper, Stack, Text, Title } from '@mantine/core';
import { RefreshCw, AlertTriangle } from 'lucide-react';

class ErrorBoundary extends React.Component {
  state = { hasError: false, error: null };

  static getDerivedStateFromError(error) {
    return { hasError: true, error };
  }

  componentDidCatch(error, errorInfo) {
    console.error('ErrorBoundary caught:', error, errorInfo);
  }

  handleRefresh = () => {
    window.location.reload();
  };

  render() {
    if (this.state.hasError) {
      const isChunkError =
        this.state.error?.message?.includes('loading dynamically imported module') ||
        this.state.error?.message?.includes('Failed to fetch dynamically imported module');

      return (
        <Center p="xl" style={{ width: '100%' }}>
          <Paper withBorder p="xl" radius="md" shadow="md" bg="rgba(255, 0, 0, 0.05)" style={{ maxWidth: 500 }}>
            <Stack align="center" gap="md">
              <AlertTriangle size={48} color="var(--mantine-color-red-6)" />
              <Title order={3}>Something went wrong</Title>
              
              {isChunkError ? (
                <>
                  <Text size="sm" ta="center">
                    A new version of the application is available. Please refresh your browser to continue.
                  </Text>
                  <Button 
                    leftSection={<RefreshCw size={16} />} 
                    color="blue" 
                    onClick={this.handleRefresh}
                  >
                    Refresh Page
                  </Button>
                </>
              ) : (
                <>
                  <Text size="sm" ta="center" c="dimmed">
                    An unexpected error occurred in this component.
                  </Text>
                  <Paper withBorder p="xs" bg="rgba(0, 0, 0, 0.2)" style={{ width: '100%' }}>
                    <Text size="xs" ff="monospace" style={{ wordBreak: 'break-all' }}>
                      {this.state.error?.message || 'Unknown error'}
                    </Text>
                  </Paper>
                  <Button variant="light" color="gray" onClick={() => this.setState({ hasError: false })}>
                    Try again
                  </Button>
                </>
              )}
            </Stack>
          </Paper>
        </Center>
      );
    }
    return this.props.children;
  }
}

export default ErrorBoundary;

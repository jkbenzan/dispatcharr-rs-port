import React, { useState, useEffect } from 'react';
import {
  Box,
  Title,
  Tabs,
  Button,
  Group,
  Text,
  Progress,
  Paper,
  Card,
  Badge,
} from '@mantine/core';
import {
  Activity,
  Settings,
  Wand2,
  CheckSquare,
  Play,
} from 'lucide-react';
import StreamsTable from '../components/tables/StreamsTable';
import API from '../api';
import useStreamsTableStore from '../store/streamsTable';
import { notifications } from '@mantine/notifications';

const StreamChecker = () => {
  const [activeTab, setActiveTab] = useState('bulk');
  const [status, setStatus] = useState({
    is_running: false,
    total: 0,
    completed: 0,
    successful: 0,
    failed: 0,
    current_stream_id: null,
    current_stream_name: null,
  });

  const selectedStreamIds = useStreamsTableStore((state) => state.selectedIds);
  const clearSelection = useStreamsTableStore((state) => state.clearSelection);

  useEffect(() => {
    let interval;
    const fetchStatus = async () => {
      try {
        const res = await API.getBulkCheckStatus();
        if (res) setStatus(res);
      } catch (e) {
        console.error('Failed to get status', e);
      }
    };

    // Initial fetch
    fetchStatus();

    interval = setInterval(fetchStatus, 2000);
    return () => clearInterval(interval);
  }, []);

  const handleStartBulkCheck = async () => {
    if (selectedStreamIds.length === 0) {
      notifications.show({
        title: 'Error',
        message: 'No streams selected for checking.',
        color: 'red',
      });
      return;
    }

    try {
      await API.startBulkCheck(selectedStreamIds);
      notifications.show({
        title: 'Started',
        message: `Started checking ${selectedStreamIds.length} streams...`,
        color: 'blue',
      });
      clearSelection();
    } catch (e) {
      notifications.show({
        title: 'Error',
        message: 'Failed to start bulk check.',
        color: 'red',
      });
    }
  };

  const progressPercent = status.total > 0 ? (status.completed / status.total) * 100 : 0;

  return (
    <Box p="md">
      <Group justify="space-between" mb="md">
        <Title order={2}>Stream Checker Engine</Title>
      </Group>

      {status.is_running && (
        <Card withBorder shadow="sm" radius="md" mb="xl" p="md">
          <Group justify="space-between" mb="xs">
            <Text fw={500} display="flex" style={{ alignItems: 'center', gap: '8px' }}>
              <Activity size={18} /> Active Bulk Check
            </Text>
            <Badge color="blue" variant="light">
              {status.completed} / {status.total} Completed
            </Badge>
          </Group>
          <Progress value={progressPercent} size="xl" radius="xl" animated mb="sm" />
          <Group justify="space-between" mt="md">
            <Text size="sm" c="dimmed">
              <strong>Testing:</strong> {status.current_stream_name || 'Initializing...'}
            </Text>
            <Group>
              <Badge color="green" variant="dot">Success: {status.successful}</Badge>
              <Badge color="red" variant="dot">Failed: {status.failed}</Badge>
            </Group>
          </Group>
        </Card>
      )}

      <Tabs value={activeTab} onChange={setActiveTab} variant="outline" radius="md">
        <Tabs.List>
          <Tabs.Tab value="bulk" leftSection={<CheckSquare size={16} />}>
            Bulk Tester
          </Tabs.Tab>
          <Tabs.Tab value="sorting" leftSection={<Settings size={16} />}>
            Sorting Rules
          </Tabs.Tab>
          <Tabs.Tab value="auto" leftSection={<Wand2 size={16} />}>
            Auto-Assignment
          </Tabs.Tab>
        </Tabs.List>

        <Tabs.Panel value="bulk" pt="xl">
          <Paper withBorder shadow="sm" p="md" radius="md" mb="xl">
            <Group justify="space-between" mb="md">
              <Box>
                <Title order={4}>Select Streams to Test</Title>
                <Text size="sm" c="dimmed">
                  Use the table below to filter streams by Account or Group, select them, and hit Start.
                  Testing takes ~30s per stream as FFmpeg analyzes the real-time bitrate.
                </Text>
              </Box>
              <Button
                leftSection={<Play size={16} />}
                color="blue"
                disabled={selectedStreamIds.length === 0 || status.is_running}
                onClick={handleStartBulkCheck}
              >
                Start Bulk Check ({selectedStreamIds.length})
              </Button>
            </Group>
          </Paper>

          {/* Render the standard StreamsTable but hide its header so it integrates nicely */}
          <Box style={{ border: '1px solid #333', borderRadius: '8px', overflow: 'hidden' }}>
             <StreamsTable />
          </Box>
        </Tabs.Panel>

        <Tabs.Panel value="sorting" pt="xl">
          <Paper withBorder shadow="sm" p="md" radius="md">
            <Title order={4}>Sorting Rules Engine (Phase 2)</Title>
            <Text size="sm" c="dimmed" mt="xs">
              Here you will be able to create scoring rules based on FFprobe metrics (Bitrate, Resolution, Codec) 
              to automatically sort streams within your channels. Coming soon!
            </Text>
          </Paper>
        </Tabs.Panel>

        <Tabs.Panel value="auto" pt="xl">
          <Paper withBorder shadow="sm" p="md" radius="md">
            <Title order={4}>Auto-Assignment Rules (Phase 3)</Title>
            <Text size="sm" c="dimmed" mt="xs">
              Create rules to automatically route newly imported M3U streams into existing channels. Coming soon!
            </Text>
          </Paper>
        </Tabs.Panel>
      </Tabs>
    </Box>
  );
};

export default StreamChecker;

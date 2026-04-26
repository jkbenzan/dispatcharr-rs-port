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
import ChannelsTable from '../components/tables/ChannelsTable';
import API from '../api';
import useChannelsTableStore from '../store/channelsTable';
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

  const selectedChannelIds = useChannelsTableStore((state) => state.selectedChannelIds) || [];
  const setSelectedChannelIds = useChannelsTableStore((state) => state.setSelectedChannelIds);
  const channels = useChannelsTableStore((state) => state.channels);

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
    if (selectedChannelIds.length === 0) {
      notifications.show({
        title: 'Error',
        message: 'No channels selected for checking.',
        color: 'red',
      });
      return;
    }

    // Extract all stream IDs from the selected channels
    const streamIdsToTest = [];
    selectedChannelIds.forEach((channelId) => {
      const channel = channels.find((c) => c.id === channelId);
      if (channel && channel.streams) {
        channel.streams.forEach((channelStream) => {
          if (channelStream.stream && channelStream.stream.id) {
            streamIdsToTest.push(channelStream.stream.id);
          }
        });
      }
    });

    if (streamIdsToTest.length === 0) {
      notifications.show({
        title: 'Notice',
        message: 'The selected channels have no streams assigned to them.',
        color: 'yellow',
      });
      return;
    }

    try {
      await API.startBulkCheck(streamIdsToTest);
      notifications.show({
        title: 'Started',
        message: `Started checking ${streamIdsToTest.length} streams across ${selectedChannelIds.length} channels...`,
        color: 'blue',
      });
      setSelectedChannelIds([]);
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
                <Title order={4}>Select Channels to Test</Title>
                <Text size="sm" c="dimmed">
                  Use the table below to select channels. The Bulk Tester will extract and test all streams assigned to the selected channels.
                  Testing takes ~30s per stream as FFmpeg analyzes the real-time bitrate.
                </Text>
              </Box>
              <Group>
                <Button
                  leftSection={<Wand2 size={16} />}
                  color="violet"
                  disabled={selectedChannelIds.length === 0 || status.is_running}
                  onClick={async () => {
                    try {
                      await API.bulkSortStreams(selectedChannelIds);
                      notifications.show({ title: 'Success', message: 'Successfully sorted streams!', color: 'green' });
                      setSelectedChannelIds([]);
                    } catch (e) {
                      notifications.show({ title: 'Error', message: 'Failed to sort streams.', color: 'red' });
                    }
                  }}
                >
                  Auto-Sort ({selectedChannelIds.length})
                </Button>
                <Button
                  leftSection={<Play size={16} />}
                  color="blue"
                  disabled={selectedChannelIds.length === 0 || status.is_running}
                  onClick={handleStartBulkCheck}
                >
                  Start Bulk Check ({selectedChannelIds.length})
                </Button>
              </Group>
            </Group>
          </Paper>

          {/* Render the standard ChannelsTable */}
          <Box style={{ border: '1px solid #333', borderRadius: '8px', overflow: 'hidden' }}>
             <ChannelsTable />
          </Box>
        </Tabs.Panel>

        <Tabs.Panel value="sorting" pt="xl">
          <Paper withBorder shadow="sm" p="md" radius="md" mb="md">
            <Group justify="space-between">
              <Box>
                <Title order={4}>Sorting Rules Engine</Title>
                <Text size="sm" c="dimmed">
                  Create rules to automatically sort streams within your channels based on FFprobe metrics.
                </Text>
              </Box>
              <Button color="blue" onClick={() => notifications.show({ message: 'Sorting rules form coming next!'})}>
                Add Rule
              </Button>
            </Group>
          </Paper>

          <Paper withBorder shadow="sm" p="md" radius="md">
             <Text c="dimmed" size="sm" fs="italic">Table of rules will be rendered here...</Text>
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

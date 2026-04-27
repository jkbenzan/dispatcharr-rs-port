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
import SortingRuleForm from '../components/forms/SortingRuleForm';
import { Table, ActionIcon, Center } from '@mantine/core';
import { SquarePen, Trash2 } from 'lucide-react';

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

  // Sorting Rules State
  const [rules, setRules] = useState([]);
  const [ruleModalOpen, setRuleModalOpen] = useState(false);
  const [editingRule, setEditingRule] = useState(null);

  const fetchRules = async () => {
    try {
      const data = await API.listSortingRules();
      if (data) setRules(data);
    } catch (e) {
      console.error('Failed to fetch sorting rules', e);
    }
  };

  useEffect(() => {
    fetchRules();
  }, []);

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
        channel.streams.forEach((streamObj) => {
          // Rust backend returns flattened stream objects directly
          if (streamObj.id) {
            streamIdsToTest.push(streamObj.id);
          } else if (streamObj.stream && streamObj.stream.id) {
            streamIdsToTest.push(streamObj.stream.id);
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
             <ChannelsTable hideLinks={true} streamCheckerMode={true} />
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
              <Button color="blue" onClick={() => { setEditingRule(null); setRuleModalOpen(true); }}>
                Add Rule
              </Button>
            </Group>
          </Paper>

          <Paper withBorder shadow="sm" p="md" radius="md">
             {rules.length === 0 ? (
               <Center p="xl"><Text c="dimmed" size="sm" fs="italic">No sorting rules defined yet. Create one above.</Text></Center>
             ) : (
               <Table striped highlightOnHover withTableBorder>
                 <Table.Thead>
                   <Table.Tr>
                     <Table.Th>Priority</Table.Th>
                     <Table.Th>Name</Table.Th>
                     <Table.Th>Condition</Table.Th>
                     <Table.Th>Score Modifier</Table.Th>
                     <Table.Th style={{ width: 100 }}>Actions</Table.Th>
                   </Table.Tr>
                 </Table.Thead>
                 <Table.Tbody>
                   {rules.sort((a,b) => a.priority - b.priority).map((rule) => (
                     <Table.Tr key={rule.id}>
                       <Table.Td>{rule.priority}</Table.Td>
                       <Table.Td fw={500}>{rule.name}</Table.Td>
                       <Table.Td>
                         <Badge variant="light" color="gray">
                           {rule.property.replace('stream_stats.', '')} {rule.operator} {rule.value}
                         </Badge>
                       </Table.Td>
                       <Table.Td>
                         <Badge color={rule.score_modifier > 0 ? 'green' : 'red'}>
                           {rule.score_modifier > 0 ? '+' : ''}{rule.score_modifier}
                         </Badge>
                       </Table.Td>
                       <Table.Td>
                         <Group gap="xs">
                           <ActionIcon variant="subtle" color="blue" onClick={() => { setEditingRule(rule); setRuleModalOpen(true); }}>
                             <SquarePen size={16} />
                           </ActionIcon>
                           <ActionIcon variant="subtle" color="red" onClick={async () => {
                             if (window.confirm('Delete this rule?')) {
                               await API.deleteSortingRule(rule.id);
                               fetchRules();
                             }
                           }}>
                             <Trash2 size={16} />
                           </ActionIcon>
                         </Group>
                       </Table.Td>
                     </Table.Tr>
                   ))}
                 </Table.Tbody>
               </Table>
             )}
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

      <SortingRuleForm 
        opened={ruleModalOpen} 
        onClose={() => setRuleModalOpen(false)} 
        rule={editingRule} 
        onSuccess={fetchRules} 
      />
    </Box>
  );
};

export default StreamChecker;

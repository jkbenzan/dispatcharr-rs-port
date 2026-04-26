import React, { useEffect } from 'react';
import { Modal, TextInput, NumberInput, Select, Button, Group, Stack } from '@mantine/core';
import { useForm } from '@mantine/form';
import API from '../../api';
import { notifications } from '@mantine/notifications';

const SortingRuleForm = ({ opened, onClose, rule, onSuccess }) => {
  const isEditing = !!rule;

  const form = useForm({
    initialValues: {
      name: '',
      priority: 0,
      property: 'stream_stats.video_resolution',
      operator: '==',
      value: '',
      score_modifier: 10,
    },
    validate: {
      name: (val) => (val.trim().length === 0 ? 'Name is required' : null),
      property: (val) => (val.trim().length === 0 ? 'Property is required' : null),
      value: (val) => (val.trim().length === 0 ? 'Value is required' : null),
    },
  });

  useEffect(() => {
    if (opened) {
      if (rule) {
        form.setValues(rule);
      } else {
        form.reset();
      }
    }
  }, [opened, rule]);

  const handleSubmit = async (values) => {
    try {
      if (isEditing) {
        await API.updateSortingRule(rule.id, values);
        notifications.show({ title: 'Success', message: 'Rule updated successfully', color: 'green' });
      } else {
        await API.createSortingRule(values);
        notifications.show({ title: 'Success', message: 'Rule created successfully', color: 'green' });
      }
      onSuccess();
      onClose();
    } catch (error) {
      notifications.show({ title: 'Error', message: 'Failed to save rule', color: 'red' });
    }
  };

  return (
    <Modal opened={opened} onClose={onClose} title={isEditing ? 'Edit Sorting Rule' : 'Create Sorting Rule'} centered>
      <form onSubmit={form.onSubmit(handleSubmit)}>
        <Stack gap="md">
          <TextInput
            label="Rule Name"
            placeholder="e.g. Prefer 1080p"
            withAsterisk
            {...form.getInputProps('name')}
          />
          <NumberInput
            label="Evaluation Priority"
            description="Lower numbers run first"
            withAsterisk
            {...form.getInputProps('priority')}
          />
          <Select
            label="Property"
            description="The FFprobe property to evaluate"
            data={[
              { value: 'video_resolution', label: 'Resolution (e.g. 1080)' },
              { value: 'video_bitrate', label: 'Video Bitrate' },
              { value: 'video_codec', label: 'Video Codec (e.g. h264)' },
              { value: 'audio_codec', label: 'Audio Codec' },
            ]}
            withAsterisk
            {...form.getInputProps('property')}
          />
          <Select
            label="Operator"
            data={[
              { value: '==', label: 'Equals (==)' },
              { value: '!=', label: 'Not Equals (!=)' },
              { value: '>=', label: 'Greater Than or Equal (>=)' },
              { value: '<=', label: 'Less Than or Equal (<=)' },
              { value: 'contains', label: 'Contains' },
            ]}
            withAsterisk
            {...form.getInputProps('operator')}
          />
          <TextInput
            label="Target Value"
            placeholder="e.g. 1080 or h264"
            withAsterisk
            {...form.getInputProps('value')}
          />
          <NumberInput
            label="Score Modifier"
            description="Amount to add (or subtract) to the stream's score if it matches"
            withAsterisk
            {...form.getInputProps('score_modifier')}
          />
          <Group justify="flex-end" mt="md">
            <Button variant="default" onClick={onClose}>Cancel</Button>
            <Button type="submit" color="blue">Save Rule</Button>
          </Group>
        </Stack>
      </form>
    </Modal>
  );
};

export default SortingRuleForm;

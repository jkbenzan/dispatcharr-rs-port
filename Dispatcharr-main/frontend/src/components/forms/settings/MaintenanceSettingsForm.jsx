import useSettingsStore from '../../../store/settings.jsx';
import React, { useEffect, useState } from 'react';
import { useForm } from '@mantine/form';
import { updateSetting } from '../../../utils/pages/SettingsUtils.js';
import {
  Alert,
  Button,
  Flex,
  NumberInput,
  Stack,
  Switch,
} from '@mantine/core';
import { MAINTENANCE_SETTINGS_OPTIONS } from '../../../constants.js';
import {
  getMaintenanceSettingDefaults,
  getMaintenanceSettingsFormInitialValues,
} from '../../../utils/forms/settings/MaintenanceSettingsFormUtils.js';

const MaintenanceSettingsOptions = React.memo(({ maintenanceSettingsForm }) => {
  const isNumericField = (key) => {
    return [
      'stream_check_frequency_days',
      'off_hours_start',
      'off_hours_end',
      'idle_threshold_minutes',
      'batch_size',
      'extended_test_duration_seconds',
    ].includes(key);
  };

  const isBooleanField = (key) => {
    return key === 'extended_test_enabled';
  };

  const getNumericFieldMax = (key) => {
    switch (key) {
      case 'off_hours_start':
      case 'off_hours_end':
        return 23;
      case 'stream_check_frequency_days':
        return 365;
      case 'batch_size':
        return 1000;
      default:
        return 10000;
    }
  };

  return (
    <>
      {Object.entries(MAINTENANCE_SETTINGS_OPTIONS).map(([key, config]) => {
        if (isNumericField(key)) {
          return (
            <NumberInput
              key={key}
              label={config.label}
              {...maintenanceSettingsForm.getInputProps(key)}
              description={config.description || null}
              min={0}
              max={getNumericFieldMax(key)}
            />
          );
        } else if (isBooleanField(key)) {
          return (
            <Switch
              key={key}
              label={config.label}
              {...maintenanceSettingsForm.getInputProps(key, { type: 'checkbox' })}
              description={config.description || null}
              mt="md"
            />
          );
        }
        return null;
      })}
    </>
  );
});

const MaintenanceSettingsForm = React.memo(({ active }) => {
  const settings = useSettingsStore((s) => s.settings);
  const [saved, setSaved] = useState(false);

  const maintenanceSettingsForm = useForm({
    mode: 'controlled',
    initialValues: getMaintenanceSettingsFormInitialValues(),
  });

  useEffect(() => {
    if (!active) setSaved(false);
  }, [active]);

  useEffect(() => {
    if (settings) {
      if (settings['maintenance_settings']?.value) {
        maintenanceSettingsForm.setValues({
          ...getMaintenanceSettingDefaults(),
          ...settings['maintenance_settings'].value,
        });
      }
    }
  }, [settings]);

  const resetMaintenanceSettingsToDefaults = () => {
    maintenanceSettingsForm.setValues(getMaintenanceSettingDefaults());
  };

  const onMaintenanceSettingsSubmit = async () => {
    setSaved(false);
    try {
      const result = await updateSetting({
        ...settings['maintenance_settings'],
        value: maintenanceSettingsForm.getValues(),
      });
      if (result) {
        setSaved(true);
      }
    } catch (error) {
      console.error('Error saving maintenance settings:', error);
    }
  };

  return (
    <form onSubmit={maintenanceSettingsForm.onSubmit(onMaintenanceSettingsSubmit)}>
      <Stack gap="sm">
        {saved && (
          <Alert
            variant="light"
            color="green"
            title="Saved Successfully"
          ></Alert>
        )}

        <MaintenanceSettingsOptions maintenanceSettingsForm={maintenanceSettingsForm} />

        <Flex mih={50} gap="xs" justify="space-between" align="flex-end">
          <Button
            variant="subtle"
            color="gray"
            onClick={resetMaintenanceSettingsToDefaults}
          >
            Reset to Defaults
          </Button>
          <Button
            type="submit"
            disabled={maintenanceSettingsForm.submitting}
            variant="default"
          >
            Save
          </Button>
        </Flex>
      </Stack>
    </form>
  );
});

export default MaintenanceSettingsForm;

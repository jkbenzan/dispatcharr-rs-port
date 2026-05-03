import { MAINTENANCE_SETTINGS_OPTIONS } from '../../../constants.js';

export const getMaintenanceSettingsFormInitialValues = () => {
  return Object.keys(MAINTENANCE_SETTINGS_OPTIONS).reduce((acc, key) => {
    acc[key] = '';
    return acc;
  }, {});
};

export const getMaintenanceSettingDefaults = () => {
  return {
    stream_check_frequency_days: 7,
    off_hours_start: 2,
    off_hours_end: 6,
    idle_threshold_minutes: 30,
    batch_size: 50,
    extended_test_enabled: false,
    extended_test_duration_seconds: 60,
  };
};

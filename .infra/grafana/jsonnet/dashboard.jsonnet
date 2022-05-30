local ds = {
  influx: {
    type: 'influxdb',
    uid: std.extVar('influxdb'),
  },
  loki: {
    type: 'loki',
    uid: std.extVar('loki'),
  },
};

local panel = {
  lyrics: import './panels/lyrics.jsonnet',
  lyrics_stats_ratios: import './panels/lyrics_stats_ratios.jsonnet',
  process_timings: import './panels/process_timings.jsonnet',
  rustify_logs: import './panels/rustify_logs.jsonnet',
  test: import './panels/test.jsonnet',
  track_status: import './panels/track_status.jsonnet',
  uptime: import './panels/uptime.jsonnet',
};

local template = {
  instance: import './templates/instance.jsonnet',
  log_level: import './templates/log_level.jsonnet',
};

{
  editable: true,
  fiscalYearStartMonth: 0,
  graphTooltip: 1,
  id: 12,
  links: [],
  liveNow: false,
  refresh: false,
  schemaVersion: 36,
  style: 'dark',
  timepicker: {},
  timezone: '',
  title: 'Rustify',
  uid: 'FiVR6lsnz',
  version: 55,
  weekStart: '',
  time: { from: 'now-6h', to: 'now' },
  annotations: {
    list: [
      {
        builtIn: 1,
        datasource: {
          type: 'datasource',
          uid: 'grafana',
        },
        enable: true,
        hide: true,
        iconColor: 'rgba(0, 211, 255, 1)',
        name: 'Annotations & Alerts',
        target: {
          limit: 100,
          matchAny: false,
          tags: [],
          type: 'dashboard',
        },
        type: 'dashboard',
      },
    ],
  },
  templating: {
    list: [
      template.instance.create(ds),
      template.log_level.create(ds),
    ],
  },
  panels: [
    {
      collapsed: true,
      id: 14,
      title: 'Test',
      type: 'row',
      gridPos: { h: 1, w: 24, x: 0, y: 0 },
      panels: [
        panel.test.create(ds),
        panel.uptime.create(ds),
      ],
    },
    {
      id: 12,
      title: 'Main',
      type: 'row',
      gridPos: { h: 1, w: 24, x: 0, y: 1 },
    },
    panel.track_status.create(ds),
    panel.process_timings.create(ds),
    panel.lyrics.create(ds),
    panel.lyrics_stats_ratios.create(ds),
    panel.rustify_logs.create(ds),
  ],
  tags: [],
}

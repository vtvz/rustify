local influx = {
  type: 'influxdb',
  uid: std.extVar('influxdb'),
};

local loki = {
  type: 'loki',
  uid: std.extVar('loki'),
};


local loki_line_format_header_items = [
  '{{ if %(path)s }}\\t%(name)s: {{ %(path)s }}{{ end }}' % item
  for item in [
    { path: '.user_id', name: 'User ID' },
    { path: '.track_id', name: 'Track ID' },
  ]
];

local loki_line_format_header = std.join('', [
  '{{ .level }} - {{ ._target }}:{{ ._line }}',
] + loki_line_format_header_items);

local loki_line_format = std.join('\\n', [
  // bold
  std.format('\\033[1;37m%s\\033[0m', loki_line_format_header),
  '',
  '{{ .message }}{{ if .err }}',
  '',
  '\\033[1;31mError content:\\033[0m',
  '{{ .err }}{{ end }}',
]);

{
  editable: true,
  fiscalYearStartMonth: 0,
  graphTooltip: 1,
  id: 12,
  iteration: 1653404727063,
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
  time: {
    from: 'now-6h',
    to: 'now',
  },
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
      {
        allValue: '',
        current: {
          selected: true,
          text: [
            'All',
          ],
          value: [
            '$__all',
          ],
        },
        datasource: loki,
        definition: 'label_values({app="rustify", instance="$instance"} , level)',
        hide: 0,
        includeAll: true,
        label: 'Log Level',
        multi: true,
        name: 'log_level',
        options: [],
        query: 'label_values({app="rustify", instance="$instance"} , level)',
        refresh: 2,
        regex: '',
        skipUrlSync: false,
        sort: 0,
        type: 'query',
      },
      {
        current: {
          selected: false,
          text: 'prod',
          value: 'prod',
        },
        datasource: loki,
        definition: 'label_values({app="rustify"} , instance)',
        hide: 0,
        includeAll: false,
        label: 'Instance',
        multi: false,
        name: 'instance',
        options: [],
        query: 'label_values({app="rustify"} , instance)',
        refresh: 1,
        regex: '',
        skipUrlSync: false,
        sort: 0,
        type: 'query',
      },
    ],
  },
  panels: [
    {
      collapsed: true,
      id: 14,
      title: 'Test',
      type: 'row',
      gridPos: {
        h: 1,
        w: 24,
        x: 0,
        y: 0,
      },
      panels: [
        {
          id: 16,
          title: 'Test',
          type: 'timeseries',
          datasource: influx,
          fieldConfig: {
            defaults: {
              color: {
                mode: 'palette-classic',
              },
              custom: {
                axisLabel: '',
                axisPlacement: 'auto',
                barAlignment: 0,
                drawStyle: 'line',
                fillOpacity: 0,
                gradientMode: 'none',
                hideFrom: {
                  legend: false,
                  tooltip: false,
                  viz: false,
                },
                lineInterpolation: 'linear',
                lineWidth: 1,
                pointSize: 5,
                scaleDistribution: {
                  type: 'linear',
                },
                showPoints: 'auto',
                spanNulls: false,
                stacking: {
                  group: 'A',
                  mode: 'none',
                },
                thresholdsStyle: {
                  mode: 'off',
                },
              },
              mappings: [],
              thresholds: {
                mode: 'absolute',
                steps: [
                  {
                    color: 'green',
                    value: null,
                  },
                  {
                    color: 'red',
                    value: 80,
                  },
                ],
              },
            },
            overrides: [
              {
                __systemRef: 'hideSeriesFrom',
                matcher: {
                  id: 'byNames',
                  options: {
                    mode: 'exclude',
                    names: [
                      'users_checked {app="rustify", instance="$instance"}',
                      'users_count {app="rustify", instance="$instance"}',
                    ],
                    prefix: 'All except:',
                    readOnly: true,
                  },
                },
                properties: [
                  {
                    id: 'custom.hideFrom',
                    value: {
                      legend: false,
                      tooltip: false,
                      viz: true,
                    },
                  },
                ],
              },
            ],
          },
          gridPos: {
            h: 8,
            w: 12,
            x: 0,
            y: 1,
          },
          options: {
            legend: {
              calcs: [],
              displayMode: 'list',
              placement: 'bottom',
            },
            tooltip: {
              mode: 'single',
              sort: 'none',
            },
          },
          targets: [
            {
              datasource: influx,
              query: |||
                data = from(bucket: v.bucket)
                  |> range(start: v.timeRangeStart, stop: v.timeRangeStop)
                  |> filter(fn: (r) => r["app"] == "rustify")
                  |> filter(fn: (r) => r["instance"] == "$instance")
                  |> filter(fn: (r) => r._measurement == "process_timings")

                data
                  |> filter(fn: (r) => r._field == "users_checked")
                  |> aggregateWindow(every: v.windowPeriod, fn: sum, createEmpty: false)
                  |> set(key: "_field", value: "users_checked")
                  |> yield(name: "users_checked")

                data
                  |> filter(fn: (r) => r._field == "users_count")
                  |> aggregateWindow(every: v.windowPeriod, fn: mean, createEmpty: false)
                  |> set(key: "_field", value: "users_count")
                  |> yield(name: "users_count")
              |||,
              refId: 'A',
            },
          ],
        },
        {
          id: 18,
          title: 'Uptime',
          type: 'timeseries',
          datasource: influx,
          fieldConfig: {
            defaults: {
              color: {
                mode: 'palette-classic',
              },
              custom: {
                axisLabel: '',
                axisPlacement: 'auto',
                barAlignment: 0,
                drawStyle: 'line',
                fillOpacity: 0,
                gradientMode: 'none',
                hideFrom: {
                  legend: false,
                  tooltip: false,
                  viz: false,
                },
                lineInterpolation: 'linear',
                lineWidth: 1,
                pointSize: 5,
                scaleDistribution: {
                  type: 'linear',
                },
                showPoints: 'auto',
                spanNulls: false,
                stacking: {
                  group: 'A',
                  mode: 'none',
                },
                thresholdsStyle: {
                  mode: 'off',
                },
              },
              mappings: [],
              thresholds: {
                mode: 'absolute',
                steps: [
                  {
                    color: 'green',
                    value: null,
                  },
                  {
                    color: 'red',
                    value: 80,
                  },
                ],
              },
              unit: 's',
            },
            overrides: [],
          },
          gridPos: {
            h: 8,
            w: 12,
            x: 12,
            y: 1,
          },
          options: {
            legend: {
              calcs: [
                'lastNotNull',
              ],
              displayMode: 'list',
              placement: 'bottom',
            },
            tooltip: {
              mode: 'single',
              sort: 'none',
            },
          },
          targets: [
            {
              datasource: influx,
              query: |||
                data = from(bucket: v.bucket)
                  |> range(start: v.timeRangeStart, stop: v.timeRangeStop)
                  |> filter(fn: (r) => r["app"] == "rustify")
                  |> filter(fn: (r) => r["instance"] == "$instance")
                  |> filter(fn: (r) => r._measurement == "uptime")

                data
                  |> aggregateWindow(every: v.windowPeriod, fn: max, createEmpty: false)
                  |> yield(name: "max")

                data
                  |> difference()
                  |> yield(name: "increase")
              |||,
              refId: 'A',
            },
          ],
        },
      ],
    },
    {
      id: 12,
      title: 'Main',
      type: 'row',
      gridPos: {
        h: 1,
        w: 24,
        x: 0,
        y: 1,
      },
    },
    {
      id: 4,
      title: 'Track Status',
      type: 'timeseries',
      datasource: influx,
      fieldConfig: {
        defaults: {
          color: {
            mode: 'palette-classic',
          },
          custom: {
            axisLabel: '',
            axisPlacement: 'auto',
            barAlignment: 0,
            drawStyle: 'line',
            fillOpacity: 0,
            gradientMode: 'none',
            hideFrom: {
              legend: false,
              tooltip: false,
              viz: false,
            },
            lineInterpolation: 'linear',
            lineWidth: 1,
            pointSize: 5,
            scaleDistribution: {
              type: 'linear',
            },
            showPoints: 'auto',
            spanNulls: false,
            stacking: {
              group: 'A',
              mode: 'none',
            },
            thresholdsStyle: {
              mode: 'off',
            },
          },
          displayName: '${__field.name}',
          mappings: [],
          thresholds: {
            mode: 'absolute',
            steps: [
              {
                color: 'green',
                value: null,
              },
              {
                color: 'red',
                value: 80,
              },
            ],
          },
        },
        overrides: [
          {
            __systemRef: 'hideSeriesFrom',
            matcher: {
              id: 'byNames',
              options: {
                mode: 'exclude',
                names: [
                  'ignored',
                  'skipped',
                  'disliked',
                ],
                prefix: 'All except:',
                readOnly: true,
              },
            },
            properties: [
              {
                id: 'custom.hideFrom',
                value: {
                  legend: false,
                  tooltip: false,
                  viz: true,
                },
              },
            ],
          },
        ],
      },
      gridPos: {
        h: 8,
        w: 12,
        x: 0,
        y: 2,
      },
      options: {
        legend: {
          calcs: [
            'lastNotNull',
          ],
          displayMode: 'list',
          placement: 'bottom',
        },
        tooltip: {
          mode: 'single',
          sort: 'none',
        },
      },
      targets: [
        {
          datasource: influx,
          query: |||
            from(bucket: v.bucket)
              |> range(start: v.timeRangeStart, stop: v.timeRangeStop)
              |> filter(fn: (r) => r["app"] == "rustify")
              |> filter(fn: (r) => r["instance"] == "$instance")
              |> filter(fn: (r) => r["_measurement"] == "track_status")
              |> aggregateWindow(every: v.windowPeriod, fn: mean, createEmpty: false)
              |> yield(name: "mean")
          |||,
          refId: 'A',
        },
      ],
    },
    {
      id: 6,
      title: 'Process Timings',
      type: 'timeseries',
      interval: '1m',
      datasource: influx,
      fieldConfig: {
        defaults: {
          color: {
            mode: 'palette-classic',
          },
          custom: {
            axisLabel: '',
            axisPlacement: 'auto',
            axisSoftMax: 3000,
            barAlignment: 0,
            drawStyle: 'line',
            fillOpacity: 0,
            gradientMode: 'none',
            hideFrom: {
              legend: false,
              tooltip: false,
              viz: false,
            },
            lineInterpolation: 'linear',
            lineStyle: {
              fill: 'solid',
            },
            lineWidth: 1,
            pointSize: 5,
            scaleDistribution: {
              type: 'linear',
            },
            showPoints: 'auto',
            spanNulls: false,
            stacking: {
              group: 'A',
              mode: 'none',
            },
            thresholdsStyle: {
              mode: 'area',
            },
          },
          displayName: '${__field.name}',
          mappings: [],
          max: 4000,
          thresholds: {
            mode: 'absolute',
            steps: [
              {
                color: 'green',
                value: null,
              },
              {
                color: 'red',
                value: 3000,
              },
            ],
          },
          unit: 'ms',
        },
        overrides: [
          {
            __systemRef: 'hideSeriesFrom',
            matcher: {
              id: 'byNames',
              options: {
                mode: 'exclude',
                names: [
                  'max',
                  '90%',
                  'min',
                ],
                prefix: 'All except:',
                readOnly: true,
              },
            },
            properties: [
              {
                id: 'custom.hideFrom',
                value: {
                  legend: false,
                  tooltip: false,
                  viz: true,
                },
              },
            ],
          },
        ],
      },
      gridPos: {
        h: 8,
        w: 12,
        x: 12,
        y: 2,
      },
      options: {
        legend: {
          calcs: [
            'lastNotNull',
          ],
          displayMode: 'list',
          placement: 'bottom',
        },
        tooltip: {
          mode: 'multi',
          sort: 'desc',
        },
      },
      targets: [
        {
          datasource: influx,
          query: |||
            data = from(bucket: v.bucket)
            	|> range(start: v.timeRangeStart, stop: v.timeRangeStop)
              |> filter(fn: (r) => r["app"] == "rustify")
              |> filter(fn: (r) => r["instance"] == "$instance")
              |> filter(fn: (r) => r._measurement == "process_timings")

            user = data
              |> filter(fn: (r) => r["_field"] == "users_process_time")

            user
              |> aggregateWindow(every: v.windowPeriod, fn: min, createEmpty: false)
              |> set(key: "_field", value: "min")
              |> yield(name: "min")

            user
              |> aggregateWindow(every: v.windowPeriod, fn: max, createEmpty: false)
              |> set(key: "_field", value: "max")
              |> yield(name: "max")

            user
              |> aggregateWindow(every: v.windowPeriod, fn: (tables=<-, column) => tables |> quantile(q: 0.90), createEmpty: false)
              |> set(key: "_field", value: "90%")
              |> yield(name: "90%")

            data
              |> filter(fn: (r) => r["_field"] == "max_process_time")
              |> aggregateWindow(every: v.windowPeriod, fn: mean, createEmpty: false)
              |> set(key: "_field", value: "limit")
          |||,
          refId: 'A',
        },
      ],
    },
    {
      id: 8,
      title: 'Lyrics',
      type: 'timeseries',
      datasource: influx,
      fieldConfig: {
        defaults: {
          color: {
            mode: 'palette-classic',
          },
          custom: {
            axisLabel: '',
            axisPlacement: 'auto',
            barAlignment: 0,
            drawStyle: 'line',
            fillOpacity: 0,
            gradientMode: 'none',
            hideFrom: {
              legend: false,
              tooltip: false,
              viz: false,
            },
            lineInterpolation: 'linear',
            lineWidth: 1,
            pointSize: 5,
            scaleDistribution: {
              type: 'linear',
            },
            showPoints: 'auto',
            spanNulls: false,
            stacking: {
              group: 'A',
              mode: 'none',
            },
            thresholdsStyle: {
              mode: 'off',
            },
          },
          displayName: '${__field.name}',
          mappings: [],
          thresholds: {
            mode: 'absolute',
            steps: [
              {
                color: 'green',
                value: null,
              },
            ],
          },
        },
        overrides: [],
      },
      gridPos: {
        h: 8,
        w: 12,
        x: 0,
        y: 10,
      },
      options: {
        legend: {
          calcs: [
            'lastNotNull',
          ],
          displayMode: 'list',
          placement: 'bottom',
        },
        tooltip: {
          mode: 'multi',
          sort: 'desc',
        },
      },
      targets: [
        {
          datasource: influx,
          query: |||
            from(bucket: v.bucket)
              |> range(start: v.timeRangeStart, stop: v.timeRangeStop)
              |> filter(fn: (r) => r["app"] == "rustify")
              |> filter(fn: (r) => r["instance"] == "$instance")
              |> filter(fn: (r) => r["_measurement"] == "lyrics")
              |> aggregateWindow(every: v.windowPeriod, fn: mean, createEmpty: false)
              |> yield(name: "mean")

          |||,
          refId: 'A',
        },
      ],
    },
    {
      id: 10,
      title: 'Lyrics Stats Ratios',
      type: 'timeseries',
      datasource: influx,
      description: '',
      fieldConfig: {
        defaults: {
          color: {
            mode: 'palette-classic',
          },
          custom: {
            axisLabel: '',
            axisPlacement: 'auto',
            barAlignment: 0,
            drawStyle: 'line',
            fillOpacity: 0,
            gradientMode: 'none',
            hideFrom: {
              legend: false,
              tooltip: false,
              viz: false,
            },
            lineInterpolation: 'smooth',
            lineStyle: {
              fill: 'solid',
            },
            lineWidth: 1,
            pointSize: 5,
            scaleDistribution: {
              type: 'linear',
            },
            showPoints: 'auto',
            spanNulls: false,
            stacking: {
              group: 'A',
              mode: 'none',
            },
            thresholdsStyle: {
              mode: 'off',
            },
          },
          mappings: [],
          thresholds: {
            mode: 'absolute',
            steps: [
              {
                color: 'green',
                value: null,
              },
            ],
          },
          unit: 'percent',
        },
        overrides: [],
      },
      gridPos: {
        h: 8,
        w: 12,
        x: 12,
        y: 10,
      },
      options: {
        legend: {
          calcs: [
            'lastNotNull',
          ],
          displayMode: 'list',
          placement: 'bottom',
        },
        tooltip: {
          mode: 'multi',
          sort: 'desc',
        },
      },
      targets: [
        {
          datasource: influx,
          query: |||
            data = from(bucket: v.bucket)
              |> range(start: v.timeRangeStart, stop: v.timeRangeStop)
              |> filter(fn: (r) => r["app"] == "rustify")
              |> filter(fn: (r) => r["instance"] == "$instance")
              |> filter(fn: (r) => r["_measurement"] == "lyrics")
              |> aggregateWindow(every: v.windowPeriod, fn: mean, createEmpty: false)
              |> pivot(rowKey: ["_time"], columnKey: ["_field"], valueColumn: "_value")

            percent = (sample, total) => if total == 0 then 0.0 else (float(v: sample) / float(v: total)) * 100.0

            data
              |> map(fn: (r) => ({ r with
                profane_ratio: percent(sample: r.profane, total: r.found),
                musixmatch_ratio: percent(sample: r.musixmatch, total: r.musixmatch + r.genius),
                found_ratio: percent(sample: r.found, total: r.checked)
              }))
              |> keep(columns: ["_time", "musixmatch_ratio", "profane_ratio", "found_ratio"])
              |> yield(name: "result")
          |||,
          refId: 'A',
        },
      ],
    },
    {
      id: 2,
      datasource: loki,
      title: 'Rustify Logs',
      type: 'logs',
      gridPos: {
        h: 15,
        w: 24,
        x: 0,
        y: 18,
      },
      options: {
        dedupStrategy: 'exact',
        enableLogDetails: true,
        prettifyLogMessage: false,
        showCommonLabels: false,
        showLabels: false,
        showTime: true,
        sortOrder: 'Descending',
        wrapLogMessage: false,
      },
      targets: [
        {
          datasource: loki,
          expr: std.format(|||
            {app="rustify", instance="$instance", level=~"$log_level"}
              | json
              | line_format "%s"
          |||, loki_line_format),
          refId: 'A',
        },
      ],
    },
  ],
  tags: [],
}

{
  create(ds): {
    id: 6,
    title: 'Process Timings',
    type: 'timeseries',
    interval: '1m',
    datasource: ds.influx,
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
    gridPos: { h: 8, w: 12, x: 12, y: 2 },
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
        datasource: ds.influx,
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
}

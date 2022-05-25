{
  create(ds): {
    id: 16,
    title: 'Test',
    type: 'timeseries',
    datasource: ds.influx,
    fieldConfig: {
      defaults: {
        displayName: '${__field.name}',
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
            viz: true,
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
                viz: false,
              },
            },
          ],
        },
      ],
    },
    gridPos: { h: 8, w: 12, x: 0, y: 1 },
    options: {
      legend: {
        calcs: [],
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
}

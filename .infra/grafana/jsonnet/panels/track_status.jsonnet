{
  create(ds): {
    id: 4,
    title: 'Track Status',
    type: 'timeseries',
    datasource: ds.influx,
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
    gridPos: { h: 12, w: 12, x: 0, y: 2 },
    options: {
      legend: {
        calcs: ['lastNotNull', 'diff'],
        displayMode: 'table',
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
}

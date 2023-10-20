{
  create(ds, gridPos): {
    id: 1,
    title: 'Tick Status',
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
      overrides: [],
    },
    gridPos: gridPos,
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
            |> filter(fn: (r) => r["_measurement"] == "tick_health")
            |> aggregateWindow(every: v.windowPeriod, fn: min, createEmpty: false)
            |> yield(name: "min")
        |||,
        refId: 'A',
      },
    ],
  },
}

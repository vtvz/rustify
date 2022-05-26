{
  create(ds): {
    id: 18,
    title: 'Uptime',
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
            viz: false,
          },
          lineInterpolation: 'smooth',
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
    gridPos: { h: 8, w: 12, x: 12, y: 1 },
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
            |> filter(fn: (r) => r._measurement == "uptime")

          data
            |> aggregateWindow(every: v.windowPeriod, fn: max, createEmpty: false)
            |> yield(name: "max")

          data
            |> difference(nonNegative: true)
            |> yield(name: "increase")
        |||,
        refId: 'A',
      },
    ],
  },
}

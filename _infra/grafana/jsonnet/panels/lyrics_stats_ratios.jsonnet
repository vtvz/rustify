{
  create(ds, gridPos): {
    id: 10,
    title: 'Lyrics Stats Ratios',
    type: 'timeseries',
    datasource: ds.influx,
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
    gridPos: gridPos,
    options: {
      legend: {
        calcs: ['lastNotNull', 'range'],
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
}

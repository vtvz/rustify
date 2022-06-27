{
  create(ds): {
    label: 'Instance',
    name: 'instance',
    datasource: ds.influx,
    query: |||
      from(bucket: v.bucket)
        |> range(start: v.timeRangeStart, stop: v.timeRangeStop)
        |> filter(fn: (r) => r["app"] == "rustify")
        |> keep(columns: ["instance"])
        |> distinct(column: "instance")
    |||,
    current: {
      selected: false,
      text: 'prod',
      value: 'prod',
    },
    hide: 0,
    includeAll: false,
    multi: false,
    options: [],
    refresh: 1,
    regex: '',
    skipUrlSync: false,
    sort: 0,
    type: 'query',
  },
}

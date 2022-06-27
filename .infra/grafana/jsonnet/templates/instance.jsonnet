{
  create(ds): {
    label: 'Instance',
    name: 'instance',
    datasource: ds.influx,
    query: |||
      import "influxdata/influxdb/v1"
      v1.tagValues(
          bucket: v.bucket,
          tag: "instance",
          predicate: (r) => true,
          start: v.timeRangeStart,
          stop: v.timeRangeStop
      )
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
    refresh: 2,
    regex: '',
    skipUrlSync: false,
    sort: 0,
    type: 'query',
  },
}

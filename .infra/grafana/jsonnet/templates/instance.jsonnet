{
  create(ds): {
    label: 'Instance',
    name: 'instance',
    datasource: ds.loki,
    query: 'label_values({app="rustify"} , instance)',
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

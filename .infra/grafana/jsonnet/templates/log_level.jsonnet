{
  create(ds): {
    label: 'Log Level',
    name: 'log_level',
    datasource: ds.loki,
    query: 'label_values({app="rustify", instance="$instance"} , level)',
    allValue: '',
    current: {
      selected: true,
      text: ['All'],
      value: ['$__all'],
    },
    hide: 0,
    includeAll: true,
    multi: true,
    options: [],
    refresh: 2,
    regex: '',
    skipUrlSync: false,
    sort: 0,
    type: 'query',
  },
}

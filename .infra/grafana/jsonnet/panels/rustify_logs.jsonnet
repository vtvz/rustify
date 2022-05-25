{
  create(ds): {
    id: 2,
    datasource: ds.loki,
    title: 'Rustify Logs',
    type: 'logs',
    gridPos: { h: 15, w: 24, x: 0, y: 18 },
    options: {
      dedupStrategy: 'exact',
      enableLogDetails: true,
      prettifyLogMessage: false,
      showCommonLabels: false,
      showLabels: false,
      showTime: true,
      sortOrder: 'Descending',
      wrapLogMessage: false,
    },
    targets: [
      {
        local line_format_header_items = [
          '{{ if %(path)s }}\\t%(name)s: {{ %(path)s }}{{ end }}' % item
          for item in [
            { path: '.user_id', name: 'User ID' },
            { path: '.track_id', name: 'Track ID' },
          ]
        ],

        local line_format_header = std.join('', [
          '{{ .level }} - {{ ._target }}:{{ ._line }}',
        ] + line_format_header_items),

        local line_format = std.join('\\n', [
          // bold
          std.format('\\033[1;37m%s\\033[0m', line_format_header),
          '',
          '{{ .message }}{{ if .err }}',
          '',
          '\\033[1;31mError content:\\033[0m',
          '{{ .err }}{{ end }}',
        ]),

        datasource: ds.loki,
        expr: std.format(|||
          {app="rustify", instance="$instance", level=~"$log_level"}
            | json
            | line_format "%s"
        |||, line_format),
        refId: 'A',
      },
    ],
  },
}

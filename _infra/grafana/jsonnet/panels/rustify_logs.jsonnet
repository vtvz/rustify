{
  create(ds, gridPos): {
    id: 2,
    datasource: ds.loki,
    title: 'Rustify Logs',
    type: 'logs',
    gridPos: gridPos,
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
          '{{ if %(path)s }}%(name)s: {{ %(path)s }}\\t{{ end }}' % item
          for item in [
            { path: '.tick_iteration', name: 'Tick' },
            { path: '.user_id', name: 'User ID' },
            { path: '.track_id', name: 'Track ID' },
            { path: '.language', name: 'Language' },
          ]
        ],

        local line_format_header = std.join('', line_format_header_items),

        local line_format = std.join('\\n', [
          // bold
          std.format('\\033[1;37m%s\\033[0m\\t%s', ['{{ .level }} - {{ ._target }}:{{ ._line }}', '{{ ._spans }}']),
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
            | json _spans="_spans"
            | label_format _spans="{{ range $i, $e := ._spans | fromJson }}{{ if $i }}:{{ end }}{{ $e }}{{ end }}"
            | json
            | line_format "%s"
        |||, line_format),
        refId: 'A',
      },
    ],
  },
}

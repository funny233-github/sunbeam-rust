#!/usr/bin/env bash
set -euo pipefail

if [ $# -eq 0 ]; then
  cat <<'MANIFEST'
{
  "title": "Hello World",
  "description": "A minimal example extension",
  "preferences": [
    { "type": "string", "name": "greeting", "title": "Greeting", "optional": true, "default": "Hello" }
  ],
  "commands": [
    {
      "name": "greet",
      "title": "Greet",
      "mode": "filter",
      "params": [
        { "type": "string", "name": "name", "title": "Name" }
      ]
    },
    {
      "name": "preview",
      "title": "Preview Greeting",
      "mode": "detail",
      "hidden": true,
      "params": [
        { "type": "string", "name": "name", "title": "Name" }
      ]
    }
  ]
}
MANIFEST
  exit 0
fi

PAYLOAD="$1"
COMMAND=$(echo "$PAYLOAD" | jq -r '.command')
PREFERENCES=$(echo "$PAYLOAD" | jq -r '.preferences // {}')
PARAMS=$(echo "$PAYLOAD" | jq -r '.params // {}')

GREETING=$(echo "$PREFERENCES" | jq -r '.greeting // "Hello"')
NAME=$(echo "$PARAMS" | jq -r '.name // "World"')

if [ "$COMMAND" = "greet" ]; then
  jq -n --arg greeting "$GREETING" --arg name "$NAME" '{
    items: [
      {
        title: "\($greeting), \($name)!",
        subtitle: "A friendly greeting",
        accessories: ["sunbeam"],
        actions: [
          { type: "copy", title: "Copy Greeting", key: "c", text: "\($greeting), \($name)!" },
          { type: "run", title: "Preview", key: "p", command: "preview", params: { name: $name } }
        ]
      }
    ],
    empty_text: "Nothing to greet",
    show_detail: false
  }'
elif [ "$COMMAND" = "preview" ]; then
  jq -n --arg greeting "$GREETING" --arg name "$NAME" '{
    markdown: "\($greeting), \($name)!\n\nThis is a detail preview for **\($name)**.",
    actions: [
      { type: "copy", title: "Copy Greeting", key: "c", text: "\($greeting), \($name)!" }
    ]
  }'
else
  echo "Unknown command: $COMMAND" >&2
  exit 1
fi

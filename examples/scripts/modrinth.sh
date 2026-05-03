#!/bin/sh

set -eu

if [ $# -eq 0 ]; then
  jq -n '{
    title: "Modrinth Browser",
    description: "Search projects from Modrinth",
    commands: [
      {
        name: "search-project",
        title: "Search Project",
        mode: "search"
      },
      {
        name: "project-detail",
        title: "Project Detail",
        mode: "detail",
        hidden: true,
        params: [
          { name: "slug", type: "string", title: "Project Slug" }
        ]
      }
    ]
  }'
  exit 0
fi

COMMAND=$(echo "$1" | jq -r '.command')

fetch_or_fail() {
  echo "$1" | jq '.' >/dev/null 2>&1 && echo "$1" || { echo "$2" >&2; jq -n "$3"; exit 1; }
}

if [ "$COMMAND" = "search-project" ]; then
  QUERY=$(echo "$1" | jq -r '.query // empty')

  if [ -z "$QUERY" ]; then
    jq -n '{
      items: [],
      emptyText: "Type a query to search Modrinth projects"
    }'
    exit 0
  fi

  RESPONSE=$(curl -s -f "https://api.modrinth.com/v2/search?query=$QUERY&limit=20" 2>/dev/null || echo "")

  if [ -z "$RESPONSE" ]; then
    echo "Failed to fetch results from Modrinth" >&2
    jq -n '{
      items: [],
      emptyText: "Failed to fetch results from Modrinth. Check your network connection."
    }'
    exit 1
  fi

  echo "$RESPONSE" | jq '{
    items: .hits | map({
      title: .title,
      subtitle: .description,
      accessories: [.project_type, (.downloads | tostring + " downloads")],
      detail: {
        text: .description
      },
      actions: [
        { title: "View Details", type: "run", key: "v", command: "project-detail", params: { slug: .slug } },
        { title: "Open in Browser", type: "open", key: "o", url: "https://modrinth.com/mod/\(.slug)" },
        { title: "Copy URL", type: "copy", key: "c", text: "https://modrinth.com/mod/\(.slug)", exit: true }
      ]
    }),
    emptyText: "No projects found for \"\(.query)\"",
    showDetail: true
  }'
fi

if [ "$COMMAND" = "project-detail" ]; then
  SLUG=$(echo "$1" | jq -r '.params.slug // empty')

  if [ -z "$SLUG" ]; then
    echo "Missing project slug" >&2
    jq -n '{ markdown: "# Error\n\nMissing project slug." }'
    exit 1
  fi

  RESPONSE=$(curl -s -f "https://api.modrinth.com/v2/project/$SLUG" 2>/dev/null || echo "")

  if [ -z "$RESPONSE" ]; then
    echo "Failed to fetch project details from Modrinth" >&2
    jq -n --arg slug "$SLUG" '{
      markdown: "# Error\n\nFailed to fetch details for project \"\($slug)\".",
      actions: [
        { title: "Open in Browser", type: "open", key: "o", url: "https://modrinth.com/mod/\($slug)" }
      ]
    }'
    exit 1
  fi

  echo "$RESPONSE" | jq '{
    markdown: .body,
    actions: [
      { title: "Open in Browser", type: "open", key: "o", url: "https://modrinth.com/mod/\(.slug)" },
      { title: "Copy URL", type: "copy", key: "c", text: "https://modrinth.com/mod/\(.slug)", exit: true }
    ]
  }'
fi

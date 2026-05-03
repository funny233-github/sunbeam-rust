//! Demonstrates the embedded extension templates (TypeScript, Python, Shell)
//! that ship with sunbeam-rust, showing their structure and how they get
//! used by the `extension create` command.
//!
//! Usage: cargo run --example embed_templates

fn main() {
    // The templates are embedded via include_str! in dispatch.rs.
    // We can't directly access them from examples, but we can create
    // temporary files using the same logic.

    println!("=== 1. Template Structure ===");
    let templates: Vec<(&str, &str)> = vec![
        ("deno", r#"#!/usr/bin/env -S deno run -A

import * as sunbeam from "https://deno.land/x/sunbeam/mod.ts";

const manifest = {
  title: "My Extension",
  description: "This is my extension",
  commands: [
    {
      name: "hi",
      title: "Say Hi",
      mode: "detail",
      params: [
        {
          name: "name",
          title: "Name",
          type: "text",
        },
      ],
    },
  ],
} as const satisfies sunbeam.Manifest;

if (Deno.args.length == 0) {
  console.log(JSON.stringify(manifest));
  Deno.exit(0);
}

const payload: sunbeam.Payload<typeof manifest> = JSON.parse(Deno.args[0]);
if (payload.command == "hi") {
  const name = payload.params.name;
  const detail: sunbeam.Detail = {
    text: `Hi ${name}!`,
    actions: [
      {
        title: "Copy Name",
        type: "copy",
        text: name,
      },
    ],
  };
  console.log(JSON.stringify(detail));
} else {
  console.error(`Unknown command: ${payload.command}`);
  Deno.exit(1);
}"#),
        ("python", r#"#!/usr/bin/env python3

import sys
import json

if len(sys.argv) == 1:
    manifest = {
        "title": "My Extension",
        "description": "This is my extension",
        "commands": [
            {
                "name": "hi",
                "title": "Say Hi",
                "mode": "detail",
                "params": [
                    {
                        "name": "name",
                        "title": "Name",
                        "type": "text"
                    }
                ]
            }
        ]
    }
    print(json.dumps(manifest))
    sys.exit(0)

payload = json.loads(sys.argv[1])
name = payload["params"]["name"]
detail = {
    "text": f"Hi {name}!",
    "actions": [
        {
            "title": "Copy Name",
            "type": "copy",
            "text": name
        }
    ]
}
print(json.dumps(detail))"#),
        ("shell", r#"#!/bin/sh

if [ "$#" -eq 0 ]; then
    jq -n '{
        title: "My Extension",
        description: "This is my extension",
        commands: [
            {
                name: "hi",
                title: "Say Hi",
                mode: "detail",
                params: [
                    {
                        name: "name",
                        title: "Name",
                        type: "text"
                    }
                ]
            }
        ]
    }'
    exit 0
fi

payload="$1"
COMMAND=$(echo "$payload" | jq -r '.command')
if [ "$COMMAND" = "hi" ]; then
    name="$(echo "$payload" | jq -r '.params.name')"
    jq -n --arg name "$name" '{
        text: "Hi \($name)!",
        actions: [
            {
                title: "Copy Name",
                type: "copy",
                text: $name
            }
        ]
    }'
else
    echo "Unknown command: $COMMAND" >&2
    exit 1
fi"#),
    ];

    for (lang, _content) in &templates {
        println!("  {} extension template:", lang);
        println!("    - Shebang: {}", match *lang {
            "deno" => "#!/usr/bin/env -S deno run -A",
            "python" => "#!/usr/bin/env python3",
            "shell" => "#!/bin/sh",
            _ => "N/A",
        });
        println!("    - Dependencies: {}", match *lang {
            "deno" => "https://deno.land/x/sunbeam/mod.ts",
            "python" => "json (stdlib)",
            "shell" => "jq",
            _ => "N/A",
        });
        println!("    - Commands: hi (detail mode with name param)");
        println!();
    }

    // ── 2. Language inference ───────────────────────────────────────────
    println!("=== 2. Language Inference ===");
    let test_names = vec![
        ("my-ext.ts", "deno"),
        ("my-ext.py", "python"),
        ("my-ext.sh", "sh"),
        ("extension", "deno"), // default
    ];

    for (name, expected) in &test_names {
        let ext = std::path::Path::new(name)
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let detected = match ext.as_str() {
            "ts" => "deno",
            "py" => "python",
            "sh" => "sh",
            _ => "deno", // default
        };
        let status = if detected == *expected { "✅" } else { "❌" };
        println!("  {status} {name} → {detected} (expected: {expected})");
    }
    println!();

    // ── 3. Template file creation (dry run - temp dir) ─────────────────
    println!("=== 3. Template File Creation ===");
    let tmp = std::env::temp_dir().join("sunbeam-embed-test");
    std::fs::create_dir_all(&tmp).ok();

    for (lang, content) in &templates {
        let filename = format!("extension.{}", if *lang == "deno" { "ts" } else if *lang == "python" { "py" } else { "sh" });
        let path = tmp.join(&filename);
        std::fs::write(&path, content).expect("write template");
        println!("  Created: {} ({} bytes)", path.display(), content.len());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).ok();
            println!("    → set executable");
        }
    }

    // Cleanup
    std::fs::remove_dir_all(&tmp).ok();
    println!("  (temp dir cleaned up)");

    println!("\n✅ Embed templates demo complete!");
}

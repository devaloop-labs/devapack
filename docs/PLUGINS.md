<div align="center">
  <img src="https://devalang.com/images/devalang-logo-min.png" alt="Devalang Logo" width="100" />
</div>

# ⚒️ Plugin Commands

This document explains the `devapack plugin` commands implemented by the tooling: how to create, build, list, delete and version plugins.

## Plugin folder layout

A generated plugin has this structure:

- `generated/plugins/<publisher>/<name>/`
  - `src/`        — source code
  - `plugin.toml` — plugin manifest (name, publisher, version, access, exports)
  - `README.md`

## Create

Create a new plugin skeleton. The command prompts for `publisher` and `name` (or accepts flags).

```bash
devapack plugin create
```

By default the generator creates: `generated/plugins/<publisher>/<name>/` with a `src/` folder and a minimal `plugin.toml`.

## Build

Build all plugins:

```bash
devapack plugin build
```

Build a single plugin

```bash
devapack plugin build <publisher>.<name>
```

Outputs:

- `generated/plugins/<publisher>/<name>/build/` — compiled output
- `output/plugin/<publisher>.<name>.tar.gz` — packaged archive (if packaging is enabled)

## List

List locally generated plugins:

```bash
devapack plugin list
```

This prints `generated/plugins/*` entries with basic metadata (name, version, build state).

## Delete

Remove a generated plugin and its build artifacts:

```bash
devapack plugin delete <publisher>.<name>
```

This deletes `generated/plugins/<publisher>/<name>/` and any `output/plugin/...` artifacts.

## Versioning

Bump the plugin semantic version in `plugin.toml`:

```bash
devapack plugin version <publisher>.<name> <major|minor|patch>

# Examples
devapack plugin version devaloop.808 major   # 1.0.0 → 2.0.0
devapack plugin version devaloop.808 minor   # 1.0.0 → 1.1.0
devapack plugin version devaloop.808 patch   # 1.0.0 → 1.0.1
```

The command updates `plugin.toml` and can optionally create a git tag / commit depending on your project config.

## `plugin.toml` — minimal manifest

A minimal `plugin.toml` example:

```toml
name = "my-plugin"
publisher = "mypublisher"
version = "0.1.0"
description = "Short description"
```

Add additional metadata (license, homepage, authors) as needed.

## How to use a built plugin in a DevaLang project

Copy the plugin build into your project's `.deva/plugins/` folder or reference the packaged artifact. Example (local copy):

```bash
# copy build folder into project
cp -r generated/plugins/mypublisher/my-plugin/build/ myproject/.deva/plugins/mypublisher/my-plugin/
```

If your runtime supports plugin references, you may add an entry in the project config (format depends on runtime):

```toml
[[plugins]]
path = "devalang://plugin/mypublisher.my-plugin"
version = "0.1.0"
```

## Debugging & troubleshooting

- Ensure `plugin.toml` exists and `entry` points to a valid source file.
- Check file permissions for `generated/plugins/...`

## Publishing

To publish a bank to the Devalang registry, you need to have an account on [devalang.com](https://devalang.com) and be logged in using the CLI.

```bash
devalang login
```

Once logged in, you can publish your bank using the following command:

```bash
devapack publish
```

You can also update an existing bank using:
  
```bash
devapack update
```
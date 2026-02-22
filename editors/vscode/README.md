# Breom Language Extension for VS Code

Provides language support for [Breom](https://github.com/ohxorud-dev/breom).

## Features

- Syntax highlighting
- Code completion
- Go to definition
- Find references
- Document symbols (Outline)
- Workspace symbol search
- Diagnostics
- Run current `.brm` file (`Breom: Run Current File`)
- Run current project (`Breom: Run Project`)

## Requirements

The Breom compiler must be installed and available in your PATH, or configure `breom.serverPath` in settings.

Run commands use `BREOM_HOME/bin/breom` by default. Set `breom.home` (or environment `BREOM_HOME`) and optionally override with `breom.cliPath`.
To set Breom Home from VSCode settings, configure `breom.home`.

Optional:

- Set `BREOM_STD_PATH` to point to your local std source directory when std is not discoverable from the workspace hierarchy.

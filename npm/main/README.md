# @licocon/ccline

High-performance Claude Code status-line renderer (`ccline`).

## Install

```bash
npm i -g @licocon/ccline
```

China npm mirror:

```bash
npm i -g @licocon/ccline --registry https://registry.npmmirror.com
```

## Fork notice

This is a fork of [Haleclipse/CCometixLine](https://github.com/Haleclipse/CCometixLine) (original author: Haleclipse).

It conflicts with `@cometix/ccline`: both install the `ccline` command and write `~/.claude/ccline/ccline`. Uninstall the upstream package first:

```bash
npm uninstall -g @cometix/ccline
```

## Claude Code

In `settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "~/.claude/ccline/ccline",
    "padding": 0
  }
}
```

## Postinstall

After install, a `postinstall` script hard-links (or copies) the platform binary to `~/.claude/ccline/ccline` (`ccline.exe` on Windows) so Claude Code can run it as the status line.

Skip that step:

```bash
CCLINE_SKIP_POSTINSTALL=1 npm i -g @licocon/ccline
```

`npm_config_loglevel=silent` only quiets postinstall logs; it does not skip the copy.

npm 11 warns that the postinstall script is not covered by `allow-scripts`. If
your npm version blocks unapproved install scripts, allow this one explicitly:

```bash
npm i -g @licocon/ccline --allow-scripts=@licocon/ccline
```

or skip the copy entirely and point `statusLine.command` at `ccline` (the global
command finds the platform binary on its own).

## Links

- GitHub: https://github.com/lilicocon/CCometixLine
- Issues: https://github.com/lilicocon/CCometixLine/issues

License: MIT

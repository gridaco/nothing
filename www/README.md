# nothing.graphics

The public website and documentation workspace for the Nothing graphics engine.

## Development

From the repository root:

```bash
pnpm install
pnpm --filter www dev
```

Production checks:

```bash
pnpm --filter www types:check
pnpm --filter www lint
pnpm --filter www build
```

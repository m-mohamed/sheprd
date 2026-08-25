# Herdr precedent

Herdr is the runtime owner: workspaces, tabs, panes, focus, persistence, live
IDs, and agent state. Sheprd uses Herdr's CLI boundary and does not reimplement
a multiplexer or persist guessed runtime IDs.

Sheprd creates or focuses a project workspace and can apply the explicit
`agent-dev` sample recipe only when creating a new workspace. The Ratatui
factory cockpit is the operator surface; HQ's Sol/Luna launcher is the bounded
parallel workflow.

Use wrappers first:

```bash
herdr status server
herdr workspace list
herdr workspace create --cwd PATH --label LABEL --focus
herdr workspace focus WORKSPACE_ID
```

Read live IDs from Herdr responses each time. Keep durable receipts and task
identity separate from ephemeral workspace IDs.

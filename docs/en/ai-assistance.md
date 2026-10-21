# Working with AI assistants

RUF is designed from the start so that AI assistants (e.g. Claude Code) can work with
it reliably.

## Skills

`.claude/skills/` contains guides that Claude Code loads automatically whenever a
matching task comes up:

| Skill | When |
|---|---|
| `ruf-app` | build an application/form with RUF and RNCL, visual check via screenshot |
| `rncl-component` | write or extend an RNCL component (checklist + scaffold) |
| `ruf-bridge` | extend the C++/Qt bridge: new widgets, signals, Qt version switches |

Other AI tools find the same rules in compact form in [`AGENTS.md`](../../AGENTS.md).

### Example prompts

- "Use RNCL to build a customer entry window with name, e-mail and a save button."
- "Create an RNCL component `RnclSlider` based on `QSlider` with a `value` property."
- "Use the new Qt 6.9 feature X in RUF, with a fallback for older versions."

## Why this works well for AIs

- **Self-describing components**: `properties()` returns name, type and description
  of every property – an AI can build forms through the property model without
  knowing each component's source code.
- **Visual checks without a display**: `QT_QPA_PLATFORM=offscreen` +
  `Widget::save_screenshot()` let an agent inspect the result as an image
  (overlaps, clipped text).
- **Robust API**: no dangling pointers, panics in callbacks are caught, clear error
  messages (missing `Application`, wrong property type …).
- **Fast feedback**: `cargo test --workspace` tests against real Qt within seconds.

## Adding your own skills

Create new skills as `.claude/skills/<name>/SKILL.md` with a YAML header (`name`,
`description`). The `description` decides when the skill gets loaded – so describe
concretely *when* it is needed.

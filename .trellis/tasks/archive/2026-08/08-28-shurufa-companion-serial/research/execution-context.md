# Execution Context — Companion Migration

This file is a compact execution aid. Owning specs remain authoritative; this
only keeps the task-specific subset small enough for sub-agent injection.

## Backend boundaries

- FyAgent is a modular monolith. Companion domain behavior belongs behind one
  private/crate-scoped service facade; Tauri commands only translate wire
  arguments/results and delegate.
- Reuse order: existing FyAgent owner -> already adopted dependency -> reviewed
  open source -> bespoke code only when necessary.
- For this task, reuse FY1111 Companion Rust modules and existing FyAgent
  shurufacli/enigo. Add only pinned `serialport = 4.8.1`; do not add a Tauri
  serial plugin, Web Serial, second LLM client, second history DB or second text
  typing engine.
- Same COM handle has exactly one native reader. Snapshot commands never read
  from the port.
- Windows is the only functional target for foreground capture/restore and
  shortcut dispatch in this Demo. Do not expand macOS scope.

## V2 frontend boundaries

- `/shurufa` remains a normal V2 page under the existing shell. Do not embed or
  import FY1111's independent app shell.
- V2 pages call native capability only through the feature-port/platform
  adapter boundary. Do not scatter direct `invoke()` calls in page code.
- Raw Tauri payloads are normalized once in the adapter into closed TypeScript
  types. Page code renders typed state and does not cast unknown wire data.
- Reuse existing V2 UI owners first: Button, Input, SecretInput, Spinner,
  InlineNotice, Badge, Collapsible and existing design tokens/control classes.
- Do not import leftover `src/components`, `src/hooks`, `src/lib` or `src/i18n`
  into V2. FY1111 frontend code is a behavior/layout reference only.
- A shortcut chord control is allowed to stay `/shurufa` page-local while it
  has no second V2 consumer.

## Stable data-flow contract

```text
serialport 4.8.1
  -> strict VKEY decoder
  -> native Companion runtime
       -> VKEY_INPUT: DryRun/Live shortcut state machine
       -> NET/LOG/PING/REC: status projection
       -> ASR DONE(seq,text): native exactly-once admission
  -> existing shurufa Agent(text)
  -> existing Responses stream/history
  -> existing enigo text typer
  -> current OS-focused text box
```

React displays and controls this flow; React does not drive the serial reader
and does not trigger the normal ASR->Agent path.

## Fast-Demo limits

- Preserve all current FY1111 Companion user features.
- Keep manual shurufa textarea/preview only as a secondary debug fallback.
- No ASR queue/backpressure system; use current single-flight Agent behavior.
- No firmware redesign unless a real `/1` protocol incompatibility is found.
- No new CI/release/installer work. Use focused tests plus one real Windows
  COM/Win32/ASR end-to-end manual acceptance.

---
status: current
audience: agent
tags: [projection, devtools, diagnostics]
supersedes: []
see-also: []
---

# Consumer developer-console projection

The developer console is a small runtime capability for games and authoring tools. It gives a pull-down game console and ASHA Studio's status/activity surface the same bounded vocabulary without turning either UI into a proof dashboard.

Rust owns runtime-originated records. `RuntimeSessionFacade.readDeveloperConsole()` returns the generated `DeveloperConsoleSnapshot` contract through the normal public bridge. Records have deterministic sequence order, severity, consumer-oriented category, source, message, correlation, authority tick/session identity, and a fixed structured detail shape. There is no generic string dispatch, authority mutation, raw state dump, or private bridge import.

The authority retains at most 128 records and admits at most 16 records for one authority tick. Older or rate-limited records increment `droppedRecordCount`. The snapshot hash binds the ordered records and retention metadata. These limits prevent a repeated resource failure from displacing gameplay or making the UI unusable.

Representative runtime records include:

- capability attachment during engine initialization;
- rejected command batches;
- unavailable or rejected presentation resources.

## Presentation and ownership

`@asha/runtime-session` provides two read-only projections:

- `projectDeveloperConsolePullDown` retains normal informational runtime traffic for a game console;
- `projectDeveloperConsoleActivity` selects warnings and errors for a compact Studio activity/status view.

Both functions return runtime entries and consumer-local UI messages in separate arrays. A Studio message such as “Saved scene locally” stays in the `localUi` channel; it is never represented as Rust authority or included in the runtime snapshot hash.

## What this is not

The developer console is current, bounded, user-facing observation. It is not:

- durable replay or audit evidence;
- a CI/proof result viewer;
- an omniscient inspection endpoint;
- an authority command or repair interface;
- storage for consumer-local notifications.

Durable replay/evidence remains in its existing typed authority paths. Diagnostics that must survive session retention belong in those artifacts, not in the developer console. Consumer-local messages remain owned by the game or Studio presentation layer.

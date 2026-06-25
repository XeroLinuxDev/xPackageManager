# Flatpak Permission Editor - Design

Date: 2026-06-25
Status: Approved (pending spec review)

## Goal

A robust, Flatseal-class flatpak permission editor built into xPackageManager: a
dedicated page where the user browses installed flatpaks and views/edits each
app's sandbox permissions (overrides), with full category parity to Flatseal.

## Decisions (locked)

- **Placement:** a **per-app "Permissions" icon button on each Installed-tab
  flatpak row**, placed between the existing **Info** and **Remove** buttons.
  Clicking it opens a **modal** (same pattern as the "Manage Flatpak Remotes"
  modal) **pre-selected to that app**. NOT a Settings button and NOT a dedicated
  left-nav page. The modal holds the Flatseal-style left app list (to switch
  apps) + right permissions panel.
- **Coverage:** full Flatseal parity (categories below).
- **Override scope:** user overrides by default (no root); a per-app **System**
  toggle writes system-wide overrides via pkexec.
- **Mechanism (approach C):** libflatpak (already a dependency) for accurate
  reads (enumerate apps, read base permissions from metadata); read override
  keyfiles for current state; **write** via the `flatpak override` CLI (user
  direct, system via pkexec), including `--reset`. Writes never hand-edit
  keyfiles, so they can't corrupt them.
- **Out of scope (YAGNI):** global "all apps" override; editing from the app
  detail view; any left-nav entry / dedicated page. Per-app reset-to-default IS
  included (standard Flatseal).

## Architecture

- `crates/xpm-flatpak/src/permissions.rs` (new): pure logic - data model,
  parsing metadata + override keyfiles, computing effective state, and building
  `flatpak override` argument vectors. No GUI, no side effects in the pure parts;
  fully unit-testable.
- `crates/xpm-flatpak/src/lib.rs`: `pub mod permissions;`.
- `crates/xpm-ui/src/main.rs`: state + callbacks (load app list, load one app's
  permissions, set toggle, add/remove list entries, apply system batch, reset).
- `crates/xpm-ui/ui/main.slint`: a per-app "Permissions" icon button on the
  Installed-tab flatpak row (between Info and Remove) + a permissions **modal**
  (gated by a `show-flatpak-perms-modal` flag, mirroring the remotes modal) and
  the backing Slint structs/properties. No `view` id, no nav entry.

## Data model

Each permission carries `{ default: bool, effective: bool }`. UI renders
`effective`; marks any toggle where `effective != default` as overridden
(resettable). List-type permissions (filesystems, bus names, env, persist) are
vectors of entries, each flagged default vs override.

Categories (parity):
- **Shares:** network, ipc
- **Sockets:** wayland, fallback-x11, x11, pulseaudio, session-bus, system-bus,
  ssh-auth, pcsc, cups, gpg-agent
- **Devices:** dri, input, usb, kvm, shm, all
- **Features:** devel, multiarch, bluetooth, canbus, per-app-dev-shm
- **Filesystems:** predefined toggles (host, host-os, host-etc, home, xdg-*
  dirs) + custom path entries with mode (ro / rw / create)
- **Bus names:** session + system, each entry `name` + policy (talk / own / see)
- **Environment:** `KEY=value` entries (set) and unset entries
- **Persistent:** relative path entries

## Read flow

Per installed app (libflatpak, user + system installations merged):
1. Base defaults: app metadata `[Context]` section via
   `InstalledRef::load_metadata` (a GKeyFile).
2. Override deltas: parse the override keyfiles -
   `~/.local/share/flatpak/overrides/<id>` (user),
   `/var/lib/flatpak/overrides/<id>` (system), and the `global` file.
3. Effective = defaults with overrides applied. `overridden` flag set where
   effective differs from default or the key is explicitly present in an override
   file. Override files are the source of truth and are re-read after every write.

Merge precedence (high -> low): per-app override > global override > metadata
default; within overrides, user > system for the current user's effective view.

## Write flow

Build a single `flatpak override` invocation per save (flags batched):
- bools -> `--share/--unshare`, `--socket/--nosocket`, `--device/--nodevice`,
  `--allow/--disallow`
- filesystems -> `--filesystem=path[:mode]` / `--nofilesystem=path`
- bus -> `--talk-name=NAME` / `--no-talk-name=NAME` / `--system-talk-name=NAME`
- env -> `--env=K=V` / `--unset-env=K`
- persist -> `--persist=path`
- reset -> `flatpak override --reset <id>` (clears all overrides for the app)

**Scope behavior:**
- **User:** apply live, per change (`flatpak override --user …`). No password.
- **System:** edits are staged; an **Apply** button runs one
  `pkexec flatpak override --system …` with all pending flags (one password
  prompt instead of one per toggle).

After any successful write, reload the app's effective state from disk.

## UI (Installed-tab row button + modal)

- **Entry point:** a "Permissions" icon button on each Installed-tab flatpak row,
  between **Info** and **Remove**. Clicking loads the installed-app list, sets the
  selected app to that row, and opens the modal (`show-flatpak-perms-modal =
  true`), same mechanism as the remotes modal.
- **Modal layout** (large, like the remotes modal but wider):
  - **Left pane:** searchable list of installed flatpaks (icon, name, app-id,
    user/system badge).
  - **Right pane:** scrollable panel with the category sections. Toggles for
    bools; add/remove row editors for filesystems, bus names, env, persist.
    Pane header: **User / System** scope selector, **Reset to defaults**, and
    (system scope) **Apply**. Overridden toggles show a small marker.
  - Modal close (X / backdrop) like other modals.

## Error handling

- App metadata unreadable -> show all defaults off + a warning; still editable.
- `flatpak` / `pkexec` failure -> inline error, then reload to reflect the real
  on-disk state (never leave the UI showing a change that didn't persist).
- Custom path validation: non-empty; absolute path or a known xdg token.

## Testing

`permissions.rs` pure-function unit tests (no GUI / no flatpak runtime):
- parse a sample metadata keyfile -> correct defaults
- parse a sample override keyfile -> correct deltas (added + removed)
- merge defaults + overrides -> correct effective state and overridden flags
- build the correct `flatpak override` arg vector for representative edits:
  add socket, remove socket, filesystem with mode, talk-name add, env set, env
  unset, reset.

## Files touched

- New: `crates/xpm-flatpak/src/permissions.rs`
- `crates/xpm-flatpak/src/lib.rs` (add `pub mod permissions;`)
- `crates/xpm-ui/src/main.rs` (state + callbacks)
- `crates/xpm-ui/ui/main.slint` (Permissions page, structs, nav entry)

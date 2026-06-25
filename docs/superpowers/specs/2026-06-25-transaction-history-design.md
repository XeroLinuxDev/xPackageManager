# Transaction History + Rollback - Design

Date: 2026-06-25
Status: Pending sign-off (safety model)

## Goal

Let users see what pacman did over time and recover from a bad update by rolling
back the *upgraded* packages of a chosen transaction - safely, without leaving a
broken or dependency-inconsistent system.

## Decisions (locked)

- **Placement:** Troubleshooting page (view 11). **Replaces the "Rebuild
  InitRAMFS" button** (initramfs is regenerated automatically by pacman hooks, so
  that action is redundant). A "Transaction History" button opens a **modal**
  (same pattern as the perms/remotes modals). No new left-nav entry.
- **v1 rollback scope:** revert the **upgraded** packages of one transaction to
  their previous versions. Undoing installs (remove) / removes (reinstall) is
  deferred to v2 (cascades dependencies; needs extra warnings).
- **No AUR. Never disable signature/security checks.**

## Architecture

- `crates/xpm-ui/src/main.rs` or a small module: pure parser of
  `/var/log/pacman.log` -> `Vec<Transaction>` (testable), plus the rollback
  orchestration (impure).
- `crates/xpm-ui/ui/main.slint`: Troubleshooting button (replacing initramfs) +
  a history modal (timeline list + per-transaction detail + rollback button).

## Data model

```
Transaction { when: String, command: String,
              actions: Vec<Action>, upgraded: usize, installed: usize, removed: usize }
Action { kind: Installed|Upgraded|Removed, pkg: String, old: String, new: String }
```

## Read flow (always safe)

Parse `/var/log/pacman.log` newest-first. Group `[ALPM] ...` action lines between
`transaction started` / `transaction completed`, attaching the nearest
`[PACMAN] Running 'pacman ...'` command. Parse the tail only (e.g. last ~100
transactions) and paginate; the log can be large. Pure function -> unit tests on
sample log text.

## Rollback flow (the only destructive part)

For a chosen transaction, target = each `Upgraded` pkg at its `old` version.

1. **Resolve each old version to a signed package file:**
   - Local cache hit: `/var/cache/pacman/pkg/<name>-<old>-<arch>.pkg.tar.zst`.
   - Else fetch from the Arch Linux Archive
     (`https://archive.archlinux.org/packages/<a>/<name>/<file>`) + its `.sig`
     into a temp dir.
   - Unresolved (not in cache or Archive) -> collected and shown; user cancels or
     proceeds without them (pacman will still refuse if that breaks deps).
2. **Critical-package guard:** if the set includes kernel (`linux*`), `glibc`,
   `systemd`, `pacman`, bootloader (`grub`/`systemd-boot`/`limine`),
   `mkinitcpio`, `nvidia*`/`mesa` -> hard warning requiring explicit confirm
   (these can break boot/login).
3. **Always-on warning + confirmation dialog (mandatory, every rollback):**
   before anything runs, a dialog states that although the operation is robust it
   cannot foresee everything, rollbacks can have unintended effects, and the user
   should have a Timeshift/btrfs snapshot first - backups are the user's
   responsibility. Buttons: Cancel / "I understand, proceed". If `snapper` or
   `timeshift` is detected, the dialog notes it; if not, it still shows the
   warning (no snapshot tool found). No auto-snapshot in v1.
4. **Apply atomically:** one `pkexec pacman -U <files...>` through the existing
   progress popup. **No** `--nodeps`, `--overwrite`, or `--noconfirm` for the
   resolution-affecting parts - pacman does full dependency resolution, shows its
   plan, and the user confirms or aborts in-app. If pacman aborts, the system is
   unchanged. Signatures are verified by pacman against the keyring.
5. Refresh installed/updates lists afterward.

## Holds: deliberately none (Option 2)

Rolled-back packages are **not** held (no `IgnorePkg`, no `pacman.conf` changes).
Reasons rejected: holding many packages creates a long-term partial-upgrade state
(fragile on a rolling distro), and a dumb user can't know which package caused the
issue, so auto-holding the whole set is wrong. Instead the rollback is framed as
short-term recovery: it stays only until the next update, and the UI tells the
user (in the entry warning and after the run) NOT to run a full update until they
have identified/fixed the cause or taken a Timeshift/Btrfs snapshot; if the newer
version still breaks, roll back again. Long-term "keep a whole update undone" is
explicitly the snapshot tools' job, mentioned only in the warning.

## Why this can't half-break the system

- Single atomic transaction; pacman refuses inconsistent sets (all-or-nothing).
- No force/nodeps flags; dependency resolution is pacman's, not ours.
- Signed packages only; verification never disabled.
- Interactive confirmation of pacman's real plan before any change.
- Missing versions stop the flow instead of producing a partial rollback.

## Out of scope (v1)

- Undoing installs/removes (v2).
- Flatpak history (`flatpak history`) - possible later.
- Automatic snapshots.

## Testing

Pure parser unit tests: sample `pacman.log` text -> correct transactions, action
kinds, version deltas, counts, and command association. Rollback file-resolution
(cache path + Archive URL construction) tested as pure helpers.

## Files touched

- `crates/xpm-ui/src/main.rs` (parser + rollback orchestration + callbacks)
- `crates/xpm-ui/ui/main.slint` (Troubleshooting button swap + history modal)

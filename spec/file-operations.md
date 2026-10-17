# File operations

## Drag and drop

| ID | Requirement | Status | Verified by |
|----|-------------|--------|-------------|
| `OPS-1` | Items can be dragged from either view. Dragging an unselected item selects it, then the whole selection is dragged. Files are offered as `text/uri-list` (`gdk::FileList`) to this app and other apps. | done | manual (tested by hand by the user, 2026-10-17) |
| `OPS-2` | A drop on a folder item puts the files in that folder. A drop on the view background puts them in the current folder. Files dropped from other apps are accepted. | partial: in the details view only the Name cell of a folder row is a drop target, and drops on its other cells go to the current folder. | manual (tested by hand by the user, 2026-10-17) |
| `OPS-3` | Drop action: Ctrl forces copy and Shift forces move. Otherwise the files are moved when they are on the same filesystem (`id::filesystem`) and copied when not. If the source doesn't allow the chosen action, copy is used if allowed, else the drop is refused. | done | manual (tested by hand by the user, 2026-10-17) |
| `OPS-4` | A drop is refused when a folder would go into itself or a descendant, or when it is a move into the folder the files are already in. | done | `ops::tests::moves_and_refuses_self_nesting` |
| `OPS-5` | Drop targets highlight with the accent colour while a drag hovers over them. | done | manual (tested by hand by the user, 2026-10-17) |

## Copy and move

| ID | Requirement | Status | Verified by |
|----|-------------|--------|-------------|
| `OPS-6` | Copies include whole folders, keep metadata, and don't follow symlinks. | done | `ops::tests::copies_directory_recursively` |
| `OPS-7` | A copy into the folder the item is already in is named `name (copy).ext`, then `name (copy 2).ext`, and so on. | done | `ops::tests::copy_into_same_dir_makes_unique_names` |
| `OPS-8` | Existing files are never overwritten without asking: a name clash goes to the conflict dialog (`OPS-11`). | done | `ops::tests::conflicts_follow_the_answer` |
| `OPS-9` | A move across filesystems is a copy, then a delete. The source is deleted only after the whole copy succeeds. | done | code review |
| `OPS-10` | After a transfer, all failures are reported together in one dialog. | done | manual |
| `OPS-11` | When a copy or move meets an item with the same name in the destination, a modal dialog asks what to do, as in Nautilus: “Replace “name”?” for a file, or “Merge folder “name”?” when both are folders, saying which folder it is already in. The buttons are Cancel, Skip, Rename and Replace (Merge for two folders). Replace deletes the existing item first, merging copies the folder's contents into the existing folder and asks again for clashes inside it, and Rename uses the name in the dialog's entry, pre-filled with the next free `name (copy).ext` name (`OPS-7`). An “Apply to all” check reuses the choice for the rest of the transfer; Rename then picks free names itself. Cancel and Escape stop the transfer, keeping what was already done. Replace is the default button, as in Nautilus. | done | `ops::tests` (each choice, with a scripted answer), `scripts/manual/transfers.sh` |
| `OPS-12` | A copy or move still running after one second shows a small progress window: “Copying “name”” or “Moving N items to “folder””, a bar of the bytes done out of the total (counted first, in the background), the amount in `g_format_size` units, and a Cancel button. Cancel stops the transfer as soon as GIO lets go of the current file, removes a partly written file, and keeps the items already done. (GIO copies a local file in one call and reports its bytes when it ends, so the bar moves file by file there.) The window closes when the transfer ends, and failures are then reported as in `OPS-10`. | done | `ops::tests` (cancel), `scripts/manual/transfers.sh` (a large file onto a slow-enough copy, screenshot, Cancel) |

## Clipboard

| ID | Requirement | Status | Verified by |
|----|-------------|--------|-------------|
| `OPS-20` | `copy` / `cut` put the selected files on the clipboard as both `x-special/gnome-copied-files` (with the copy/cut flag) and `text/uri-list`. | done | `scripts/manual/clipboard.sh` (wl-clipboard as the other app) |
| `OPS-21` | `paste` into the current folder reads `x-special/gnome-copied-files` first, so cut works with Nautilus, Thunar, Nemo and Caja. Otherwise it reads a file list and copies. | done | `clipboard::tests`, `scripts/manual/clipboard.sh` |
| `OPS-22` | After a cut is pasted, the clipboard is cleared. | done | `scripts/manual/clipboard.sh` |

## Trash, delete, rename, create

| ID | Requirement | Status | Verified by |
|----|-------------|--------|-------------|
| `OPS-30` | `trash` moves the selection to the XDG trash (`g_file_trash`). Failures are reported in one dialog. | done | manual |
| `OPS-31` | `delete` permanently deletes the selection after a confirmation dialog (“Permanently delete “name”?” or “Permanently delete N items?”) with Cancel and Delete buttons. Cancel is the default, and Escape cancels. Folders are deleted with everything in them. Failures are reported together in one dialog. | done | `ops::tests`, `scripts/manual/item-actions.sh` |
| `OPS-32` | `rename` opens a small modal box when exactly one item is selected. It is pre-filled with the name, and only the part before the extension is selected; dotfiles and names without a dot are fully selected. Enter applies the new name and Escape cancels. An empty or unchanged name does nothing. | done | `rename::tests::stem_selection`, manual |
| `OPS-33` | Rename never overwrites an existing file. A failure is shown in a dialog. | done | `rename::tests::rename_refuses_to_overwrite` |
| `OPS-34` | `new-folder` asks for a name in a small modal box like rename (`OPS-32`), filled with “Untitled Folder” (as in Nautilus) and fully selected. Enter creates the folder in the current folder, then it is selected and focused, as in `NAV-15`. Escape or an empty name cancels. A name containing `/`, or one that already exists, creates nothing, and the error is shown in a dialog. | done | `scripts/manual/item-actions.sh` |
| `OPS-35` | Right-clicking opens a context menu at the pointer. Right-clicking an item that isn't selected selects only that item first, and right-clicking empty space clears the selection, as in Nautilus. The `context-menu` binding opens the same menu at the focused item. With items selected the menu offers Open, Open With…, Cut, Copy, Rename (one item only), Move to Trash, Delete Permanently and Properties. On empty space, or with nothing selected, it offers New Folder, Paste, Select All, Show Hidden Files (a check item), Sort By (`VIEW-13`) and Properties of the current folder. Each entry shows its first configured key. The menu always opens at its full size: when it doesn't fit below or right of the pointer it opens above or to the left, or shifts along the screen edge. It scrolls only when it is taller than the screen. | done | `scripts/manual/item-actions.sh` (keyboard; right-click with wlrctl near each window edge, screenshots show no scrollbar) |
| `OPS-36` | `open-with` opens a dialog listing the apps that handle the first selected item's type (`gio::AppInfo`), with the default app first and marked. Activating an app opens all selected files in it. Escape cancels. `properties` opens a read-only window for the selection, or for the current folder when nothing is selected. It shows the name, type, location, modified time, permissions (`VIEW-35`) and owner. Files show their size. Folders show their item count and the total size of everything inside, which is counted in the background. Several items show their count and total size. Escape closes it. | done | `scripts/manual/item-actions.sh` (screenshots; a stand-in default app records the file it opens) |

## Implementation notes

- All operations are async GIO futures on the main context (`glib::spawn_future_local`).
- `OPS-11`, `OPS-12`: each copy or move is an `ops::Job` with the conflict asker, the remembered "apply to all" answer, a `gio::Cancellable` and byte counts. Copies use `copy_async`/`move_async` through `gio::GioFuture` for progress, with the job's cancellable. The progress window's one-second wait is paused while a conflict dialog is open.
- `OPS-3` checks the filesystem with a synchronous local `query_info`, cached per (source, destination) pair. Motion events fire constantly.
- `g_file_move` returns `WOULD_RECURSE` for a folder on another filesystem, which triggers the copy-then-delete path (`OPS-9`).

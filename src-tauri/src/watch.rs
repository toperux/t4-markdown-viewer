use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// Editors emit a burst of events for a single save (write temp, rename, touch).
const DEBOUNCE: Duration = Duration::from_millis(150);

/// Dropping this stops the watcher; the worker thread exits when its channel closes.
pub struct Handle {
    _watcher: RecommendedWatcher,
}

fn is_content_change(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) | EventKind::Any
    )
}

/// Watch `dirs`, emitting `event` with the list of changed paths when anything
/// `filter` claims moves. `filter` answers with the paths to *report* for an
/// event path — empty for "not mine" — because the frontend matches what it
/// registered, which is not always how the OS spells it back. Directories are
/// watched rather than files: most editors save by writing a temp file and
/// renaming over the target, which destroys a watch registered on the original
/// file.
///
/// `to` addresses a single window by label — each window watches only the files
/// its own tabs have open. `None` broadcasts, which is what the theme watcher
/// wants since a theme edit affects every window at once.
pub fn watch<F>(
    app: AppHandle,
    dirs: Vec<PathBuf>,
    filter: F,
    event: &'static str,
    to: Option<String>,
) -> Option<Handle>
where
    F: Fn(&Path) -> Vec<String> + Send + 'static,
{
    if dirs.is_empty() {
        return None;
    }

    let (tx, rx) = channel::<notify::Result<notify::Event>>();
    let mut watcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })
    .ok()?;

    let mut watching_any = false;
    for dir in &dirs {
        if watcher.watch(dir, RecursiveMode::NonRecursive).is_ok() {
            watching_any = true;
        }
    }
    if !watching_any {
        return None;
    }

    let collect = move |res: &notify::Result<notify::Event>, out: &mut Vec<String>| {
        let Ok(ev) = res else { return };
        if !is_content_change(&ev.kind) {
            return;
        }
        for p in &ev.paths {
            for path in filter(p) {
                if !out.contains(&path) {
                    out.push(path);
                }
            }
        }
    };

    std::thread::spawn(move || loop {
        let first = match rx.recv() {
            Ok(ev) => ev,
            Err(_) => return, // watcher dropped
        };
        let mut hits = Vec::new();
        collect(&first, &mut hits);

        // Drain the burst before reacting.
        loop {
            match rx.recv_timeout(DEBOUNCE) {
                Ok(ev) => collect(&ev, &mut hits),
                Err(RecvTimeoutError::Timeout) => break,
                // Every navigation rebuilds the watcher — once before the read,
                // again once the document's pictures are known — and returning
                // here threw away hits already collected, so a save landing
                // between the two was neither read nor reported. Break instead
                // and the burst is emitted; the outer `recv` then sees the
                // closed channel and the thread exits as before.
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }

        if hits.is_empty() {
            continue;
        }
        match &to {
            Some(label) => {
                let _ = app.emit_to(label, event, hits);
            }
            None => {
                let _ = app.emit(event, hits);
            }
        }
    });

    Some(Handle { _watcher: watcher })
}

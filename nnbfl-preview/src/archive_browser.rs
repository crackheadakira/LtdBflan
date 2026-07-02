use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};

use nnbfl::{
    core::ReadWriteable,
    sarc::file::{MagicFiles, Sarc, SarcFile},
};

const MAX_RECURSION_DEPTH: u32 = 32;

#[derive(Clone, Debug)]
pub struct ArchiveEntry {
    pub path: PathBuf,
    pub nested_path: Vec<usize>,
    pub display_name: String,
    /// The name of the innermost SARC entry, used for matching PartsPane's `o_layout_name`.
    pub last_label: Option<String>,
}

impl ArchiveEntry {
    pub fn layout_key(&self) -> String {
        let raw = self.last_label.clone().unwrap_or_else(|| {
            self.path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        });

        match raw.rsplit_once('.') {
            Some((stem, _ext)) => stem.to_lowercase(),
            None => raw.to_lowercase(),
        }
    }
}

enum ArchiveScanEvent {
    Found(ArchiveEntry),
    Progress { scanned: usize, total: usize },
    Finished,
}

pub struct ArchiveScan {
    root: PathBuf,
    rx: mpsc::Receiver<ArchiveScanEvent>,
    cancel: Arc<AtomicBool>,
    pub entries: Vec<ArchiveEntry>,
    pub scanned: usize,
    pub total: usize,
    pub done: bool,
    pub cancelled: bool,
}

impl ArchiveScan {
    pub fn start(root: PathBuf) -> Self {
        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_thread = Arc::clone(&cancel);
        let thread_root = root.clone();

        std::thread::spawn(move || {
            let candidates = collect_candidate_files(&thread_root);
            let total = candidates.len();
            let _ = tx.send(ArchiveScanEvent::Progress { scanned: 0, total });

            for (i, path) in candidates.into_iter().enumerate() {
                if cancel_thread.load(Ordering::Relaxed) {
                    break;
                }

                for (nested_path, labels) in find_bflyt_packages(&path) {
                    let relative = path
                        .strip_prefix(&thread_root)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .to_string();

                    let last_label = labels.last().cloned();
                    let display_name = if labels.is_empty() {
                        relative
                    } else {
                        format!("{relative} -> {}", labels.join(" -> "))
                    };

                    let _ = tx.send(ArchiveScanEvent::Found(ArchiveEntry {
                        path: path.clone(),
                        nested_path,
                        display_name,
                        last_label,
                    }));
                }

                let _ = tx.send(ArchiveScanEvent::Progress {
                    scanned: i + 1,
                    total,
                });
            }

            let _ = tx.send(ArchiveScanEvent::Finished);
        });

        Self {
            root,
            rx,
            cancel,
            entries: Vec::new(),
            scanned: 0,
            total: 0,
            done: false,
            cancelled: false,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn request_cancel(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.cancelled = true;
    }

    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        let mut found_new = false;

        while let Ok(event) = self.rx.try_recv() {
            changed = true;
            match event {
                ArchiveScanEvent::Found(entry) => {
                    self.entries.push(entry);
                    found_new = true;
                }
                ArchiveScanEvent::Progress { scanned, total } => {
                    self.scanned = scanned;
                    self.total = total;
                }
                ArchiveScanEvent::Finished => self.done = true,
            }
        }

        if found_new {
            self.entries.sort_by(|a, b| {
                a.display_name
                    .to_lowercase()
                    .cmp(&b.display_name.to_lowercase())
            });
        }

        changed
    }
}

fn collect_candidate_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let Ok(read_dir) = std::fs::read_dir(&dir) else {
            continue;
        };

        for entry in read_dir.flatten() {
            let path = entry.path();

            if path.is_dir() {
                stack.push(path);
                continue;
            }

            if peek_is_container_or_bflyt(&path) {
                out.push(path);
            }
        }
    }

    out
}

fn peek_is_container_or_bflyt(path: &Path) -> bool {
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };

    let mut header = [0u8; 4];
    if file.read_exact(&mut header).is_err() {
        return false;
    }

    matches!(&header, b"FLYT" | b"SARC" | b"Yaz0") || header == [0x28, 0xB5, 0x2F, 0xFD]
}

fn is_bflyt(data: &[u8]) -> bool {
    data.len() >= 4 && &data[0..4] == b"FLYT"
}

fn unwrap_compression(mut data: Vec<u8>, origin: &Path, depth: u32) -> Option<Vec<u8>> {
    if depth >= MAX_RECURSION_DEPTH {
        log::warn!(
            "Archive scan: hit max nesting depth ({MAX_RECURSION_DEPTH}) in {}",
            origin.display()
        );
        return Some(data);
    }

    loop {
        let probe = SarcFile {
            name: None,
            hash: 0,
            data: data.clone(),
        };

        match probe.match_by_magic() {
            MagicFiles::Zstd(compressed) => {
                let mut decompressed = Vec::new();
                if tomolib::formats::zs::decompress(&compressed[..], &mut decompressed).is_err() {
                    log::warn!(
                        "Archive scan: Zstd decompress failed in {}",
                        origin.display()
                    );
                    return None;
                }
                data = decompressed;
            }

            MagicFiles::Yaz0(compressed) => match szs::decode(&compressed) {
                Ok(decompressed) => data = decompressed,
                Err(err) => {
                    log::warn!(
                        "Archive scan: Yaz0 decode failed in {}: {err}",
                        origin.display()
                    );
                    return None;
                }
            },
            _ => return Some(data),
        }
    }
}

fn find_bflyt_packages(path: &Path) -> Vec<(Vec<usize>, Vec<String>)> {
    let mut out = Vec::new();

    let Ok(bytes) = std::fs::read(path) else {
        log::warn!("Archive scan: failed to read {}", path.display());
        return out;
    };

    let Some(bytes) = unwrap_compression(bytes, path, 0) else {
        return out;
    };

    if is_bflyt(&bytes) {
        out.push((Vec::new(), Vec::new()));
        return out;
    }

    let mut nested_path = Vec::new();
    let mut labels = Vec::new();
    walk_sarc_for_packages(&bytes, path, 0, &mut nested_path, &mut labels, &mut out);

    out
}

fn walk_sarc_for_packages(
    data: &[u8],
    origin: &Path,
    depth: u32,
    nested_path: &mut Vec<usize>,
    labels: &mut Vec<String>,
    out: &mut Vec<(Vec<usize>, Vec<String>)>,
) {
    if depth >= MAX_RECURSION_DEPTH {
        log::warn!(
            "Archive scan: hit max nesting depth ({MAX_RECURSION_DEPTH}) in {}, giving up on this branch",
            origin.display()
        );
        return;
    }

    let probe = SarcFile {
        name: None,
        hash: 0,
        data: data.to_vec(),
    };

    let MagicFiles::Sarc(sarc_bytes) = probe.match_by_magic() else {
        return;
    };

    let sarc = match Sarc::parse(&sarc_bytes) {
        Ok(sarc) => sarc,
        Err(err) => {
            log::warn!(
                "Archive scan: SARC parse failed in {}: {err:?}",
                origin.display()
            );
            return;
        }
    };

    if sarc.files.iter().any(|f| is_bflyt(&f.data)) {
        out.push((nested_path.clone(), labels.clone()));
        return;
    }

    for (i, file) in sarc.files.iter().enumerate() {
        let Some(unwrapped) = unwrap_compression(file.data.clone(), origin, depth + 1) else {
            continue;
        };

        if !matches!(
            SarcFile {
                name: None,
                hash: 0,
                data: unwrapped.clone(),
            }
            .match_by_magic(),
            MagicFiles::Sarc(_)
        ) {
            continue;
        }

        let label = file
            .name
            .clone()
            .unwrap_or_else(|| format!("#{i} (0x{:08X})", file.hash));

        nested_path.push(i);
        labels.push(label);

        walk_sarc_for_packages(&unwrapped, origin, depth + 1, nested_path, labels, out);

        labels.pop();
        nested_path.pop();
    }
}

/// Re-resolves one specific package identified by `entry.nested_path`.
pub fn resolve_nested_package_bytes(
    top_level_bytes: Vec<u8>,
    nested_path: &[usize],
) -> Option<Vec<u8>> {
    let origin = Path::new("<selected archive entry>");
    let mut data = unwrap_compression(top_level_bytes, origin, 0)?;

    for &idx in nested_path {
        let probe = SarcFile {
            name: None,
            hash: 0,
            data: data.clone(),
        };

        let MagicFiles::Sarc(sarc_bytes) = probe.match_by_magic() else {
            return None;
        };

        let sarc = Sarc::parse(&sarc_bytes).ok()?;
        let child = sarc.files.get(idx)?;
        data = unwrap_compression(child.data.clone(), origin, 0)?;
    }

    Some(data)
}

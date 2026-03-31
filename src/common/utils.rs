use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use ignore::overrides::OverrideBuilder;
use ignore::{Walk, WalkBuilder, overrides};

use crate::common::data::{IndexDiff, IndexStatus};

pub fn system_time_to_timestamp(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

pub fn hash_str(s: &str) -> String {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

pub fn calculate_diff<'a>(curr: &'a [IndexStatus], stored: &'a [IndexStatus]) -> IndexDiff<'a> {
    let curr_map: HashMap<&str, &IndexStatus> = curr
        .iter()
        .map(|status| (status.file_path.as_str(), status))
        .collect();
    let stored_map: HashMap<&str, &IndexStatus> = stored
        .iter()
        .map(|status| (status.file_path.as_str(), status))
        .collect();

    let deleted: Vec<&IndexStatus> = stored
        .iter()
        .filter(|status| !curr_map.contains_key(status.file_path.as_str()))
        .collect();
    let new: Vec<&IndexStatus> = curr
        .iter()
        .filter(|status| !stored_map.contains_key(status.file_path.as_str()))
        .collect();
    let updated: Vec<&IndexStatus> = curr
        .iter()
        .filter(|status| {
            if let Some(stored_status) = stored_map.get(status.file_path.as_str()) {
                status.is_changed(stored_status)
            } else {
                false
            }
        })
        .collect();

    IndexDiff {
        deleted,
        new,
        updated,
    }
}

pub fn construct_walker(
    path: &Path,
    case_insensitive: bool,
    includes: &[String],
    excludes: &[String],
    max_depth: Option<usize>,
) -> Walk {
    let mut builder = OverrideBuilder::new(path);
    let mut overrides = builder.case_insensitive(case_insensitive);

    for include in includes {
        overrides = overrides.and_then(|o| o.add(include));
    }

    for exclude in excludes {
        overrides = overrides.and_then(|o| o.add(&format!("!{}", exclude)));
    }

    let overrides = overrides.and_then(|o| o.build());
    match overrides {
        Ok(overrides) => WalkBuilder::new(path)
            .overrides(overrides)
            .max_depth(max_depth)
            .build(),
        Err(_) => WalkBuilder::new(path).max_depth(max_depth).build(),
    }
}

pub fn get_relative_path(path: &Path, base: &Path) -> Result<PathBuf, String> {
    let p1 = path.canonicalize().map_err(|e| e.to_string())?;
    let p2 = base.canonicalize().map_err(|e| e.to_string())?;
    p1.strip_prefix(p2)
        .map(|p| p.to_path_buf())
        .map_err(|e| e.to_string())
}

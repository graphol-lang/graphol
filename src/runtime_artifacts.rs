use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub(crate) fn find_runtime_artifacts() -> io::Result<(PathBuf, Option<PathBuf>)> {
    let exe = env::current_exe()?;
    let exe_dir = exe.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "failed to resolve executable directory",
        )
    })?;

    for candidate in runtime_candidates(exe_dir) {
        if let Some(artifacts) = resolve_runtime_artifacts(&candidate)? {
            return Ok(artifacts);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "missing runtime artifacts; run `cargo build` first or set GRAPHOL_RUNTIME_DIR",
    ))
}

fn runtime_candidates(exe_dir: &Path) -> Vec<PathBuf> {
    let profile_dir = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let manifest_target = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(profile_dir);

    let mut candidates = Vec::new();
    if let Some(custom) = env::var_os("GRAPHOL_RUNTIME_DIR") {
        candidates.push(PathBuf::from(custom));
    }
    candidates.push(exe_dir.to_path_buf());
    candidates.push(manifest_target);
    candidates
}

fn resolve_runtime_artifacts(base: &Path) -> io::Result<Option<(PathBuf, Option<PathBuf>)>> {
    let plain_rlib = base.join("libgraphol.rlib");
    if plain_rlib.is_file() {
        let deps = base.join("deps");
        return Ok(Some((plain_rlib, deps.is_dir().then_some(deps))));
    }

    let deps_path = if base.file_name().and_then(|name| name.to_str()) == Some("deps") {
        base.to_path_buf()
    } else {
        base.join("deps")
    };
    if !deps_path.is_dir() {
        return Ok(None);
    }

    let mut hashed_rlibs = Vec::new();
    for entry in fs::read_dir(&deps_path)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|file| file.to_str()) {
            if name.starts_with("libgraphol-") && name.ends_with(".rlib") {
                hashed_rlibs.push(path);
            }
        }
    }
    hashed_rlibs.sort_unstable();

    Ok(hashed_rlibs
        .into_iter()
        .next()
        .map(|rlib| (rlib, Some(deps_path))))
}

#[cfg(test)]
mod tests {
    use super::resolve_runtime_artifacts;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn resolves_plain_rlib_without_deps_dir() {
        let root = temp_dir("plain");
        fs::write(root.join("libgraphol.rlib"), b"rlib").expect("rlib should be written");

        let (rlib, deps) = resolve_runtime_artifacts(&root)
            .expect("resolution should work")
            .expect("artifacts should exist");
        assert_eq!(rlib, root.join("libgraphol.rlib"));
        assert!(deps.is_none());
    }

    #[test]
    fn resolves_hashed_rlib_inside_deps_dir() {
        let root = temp_dir("hashed");
        let deps = root.join("deps");
        fs::create_dir_all(&deps).expect("deps should be created");
        fs::write(deps.join("libgraphol-abcd1234.rlib"), b"rlib")
            .expect("hashed rlib should be written");

        let (rlib, deps) = resolve_runtime_artifacts(&root)
            .expect("resolution should work")
            .expect("artifacts should exist");
        assert_eq!(rlib, root.join("deps/libgraphol-abcd1234.rlib"));
        assert_eq!(deps, Some(root.join("deps")));
    }

    fn temp_dir(name: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be monotonic")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("graphol_artifacts_{}_{}", name, now));
        fs::create_dir_all(&path).expect("temp dir should be created");
        path
    }
}

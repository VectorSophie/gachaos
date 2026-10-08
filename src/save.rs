use std::path::Path;

use crate::game::State;

/// Loads the save, or starts fresh. A corrupt file is kept as `save.json.bad`
/// rather than silently overwritten.
pub fn load(path: &Path) -> State {
    let Ok(text) = std::fs::read_to_string(path) else { return State::default() };
    match serde_json::from_str::<State>(&text) {
        Ok(mut st) => {
            st.migrate();
            st
        }
        Err(e) => {
            eprintln!("gachaos: save file unreadable ({e}), starting fresh");
            let _ = std::fs::rename(path, path.with_extension("json.bad"));
            State::default()
        }
    }
}

/// Writes via a temp file + rename so a crash mid-write can't corrupt the save.
pub fn store(path: &Path, st: &State) {
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(st).expect("state serializes");
    if let Err(e) = std::fs::write(&tmp, json).and_then(|_| std::fs::rename(&tmp, path)) {
        eprintln!("gachaos: couldn't save to {}: {e}", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_then_load_and_corrupt_files_are_set_aside() {
        let dir = std::env::temp_dir().join(format!("gachaos-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("save.json");
        let st = State { tickets: 5, pity: 3, ..State::default() };
        store(&path, &st);
        assert_eq!(load(&path), st);

        std::fs::write(&path, "{not json").unwrap();
        assert_eq!(load(&path), State::default());
        assert!(dir.join("save.json.bad").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

use super::*;

use std::{
  collections::HashSet,
  env, fs,
  path::{Path, PathBuf},
};

#[derive(Debug)]
pub(crate) struct Bookmarks {
  entries: Vec<ListEntry>,
  ids: HashSet<String>,
  path: PathBuf,
}

impl Bookmarks {
  fn bookmarks_path() -> Result<PathBuf> {
    if let Ok(path) = env::var("HN_BOOKMARKS_FILE") {
      return Ok(PathBuf::from(path));
    }

    let base_dir = if let Ok(dir) = env::var("XDG_CONFIG_HOME") {
      PathBuf::from(dir)
    } else if let Ok(home) = env::var("HOME") {
      PathBuf::from(home).join(".config")
    } else {
      env::current_dir()?.join(".config")
    };

    Ok(base_dir.join("hn").join("bookmarks.json"))
  }

  fn ensure_parent_dir(path: &Path) -> Result {
    if let Some(parent) = path.parent() {
      fs::create_dir_all(parent)?;
    }

    Ok(())
  }

  pub(crate) fn entries(&self) -> &[ListEntry] {
    &self.entries
  }

  pub(crate) fn entries_vec(&self) -> Vec<ListEntry> {
    self.entries.clone()
  }

  pub(crate) fn is_empty(&self) -> bool {
    self.entries.is_empty()
  }

  pub(crate) fn load() -> Result<Self> {
    let path = Self::bookmarks_path()?;

    let entries = if path.exists() {
      let data = fs::read(&path)?;

      if data.is_empty() {
        Vec::new()
      } else {
        serde_json::from_slice::<Vec<ListEntry>>(&data)?
      }
    } else {
      Vec::new()
    };

    let ids = entries
      .iter()
      .map(|entry| entry.id.clone())
      .collect::<HashSet<_>>();

    Ok(Self { entries, ids, path })
  }

  pub(crate) fn remove(&mut self, id: &str) -> Result<bool> {
    if let Some(pos) = self.entries.iter().position(|entry| entry.id == id) {
      self.entries.remove(pos);
      self.ids.remove(id);
      self.persist()?;
      Ok(true)
    } else {
      Ok(false)
    }
  }

  fn persist(&self) -> Result {
    Self::ensure_parent_dir(&self.path)?;

    let serialized = serde_json::to_vec_pretty(&self.entries)?;

    fs::write(&self.path, serialized)?;

    Ok(())
  }

  pub(crate) fn toggle(&mut self, entry: &ListEntry) -> Result<bool> {
    if self.ids.contains(&entry.id) {
      self.remove(&entry.id)?;
      Ok(false)
    } else {
      self.entries.insert(0, entry.clone());
      self.ids.insert(entry.id.clone());
      self.persist()?;
      Ok(true)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use std::{
    fs,
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
  };

  static COUNTER: AtomicUsize = AtomicUsize::new(0);

  fn temp_bookmarks_file() -> PathBuf {
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    env::temp_dir().join(format!("hn_bookmarks_test_{unique}.json"))
  }

  fn with_temp_env<F>(f: F)
  where
    F: FnOnce(&Path),
  {
    let path = temp_bookmarks_file();

    unsafe {
      env::set_var("HN_BOOKMARKS_FILE", &path);
    }

    f(&path);

    unsafe {
      env::remove_var("HN_BOOKMARKS_FILE");
    }

    let _ = fs::remove_file(&path);
  }

  fn sample_entry(id: &str) -> ListEntry {
    ListEntry {
      detail: Some("detail".to_string()),
      id: id.to_string(),
      title: format!("Entry {id}"),
      url: Some(format!("https://example.com/{id}")),
    }
  }

  #[test]
  fn toggle_adds_and_removes_entries() {
    with_temp_env(|_| {
      let mut bookmarks = Bookmarks::load().unwrap();
      assert!(bookmarks.is_empty());

      let entry = sample_entry("1");
      assert!(bookmarks.toggle(&entry).unwrap());
      assert!(!bookmarks.is_empty());
      assert_eq!(bookmarks.entries()[0].id, "1");

      assert!(!bookmarks.toggle(&entry).unwrap());
      assert!(bookmarks.is_empty());
    });
  }

  #[test]
  fn remove_deletes_existing_entry() {
    with_temp_env(|path| {
      let mut bookmarks = Bookmarks::load().unwrap();
      let entry = sample_entry("2");
      bookmarks.toggle(&entry).unwrap();

      assert!(bookmarks.remove("2").unwrap());
      assert!(bookmarks.is_empty());
      assert!(fs::metadata(path).is_ok(), "file should exist");
    });
  }
}

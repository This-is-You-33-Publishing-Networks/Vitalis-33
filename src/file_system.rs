//! Virtual File System — v369
//! In-memory VFS with path manipulation, metadata, directory support.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum FsEntry {
    File { content: Vec<u8>, size: usize },
    Directory { children: Vec<String> },
}

#[derive(Debug)]
pub struct VirtualFs {
    pub entries: HashMap<String, FsEntry>,
}

impl VirtualFs {
    pub fn new() -> Self {
        let mut entries = HashMap::new();
        entries.insert("/".to_string(), FsEntry::Directory { children: Vec::new() });
        Self { entries }
    }

    pub fn normalize_path(path: &str) -> String {
        let mut parts: Vec<&str> = Vec::new();
        for part in path.split('/') {
            match part {
                "" | "." => {}
                ".." => { parts.pop(); }
                p => parts.push(p),
            }
        }
        format!("/{}", parts.join("/"))
    }

    pub fn parent_path(path: &str) -> String {
        let norm = Self::normalize_path(path);
        if norm == "/" {
            return "/".to_string();
        }
        match norm.rfind('/') {
            Some(0) => "/".to_string(),
            Some(i) => norm[..i].to_string(),
            None => "/".to_string(),
        }
    }

    pub fn file_name(path: &str) -> String {
        let norm = Self::normalize_path(path);
        norm.rsplit('/').next().unwrap_or("").to_string()
    }

    pub fn mkdir(&mut self, path: &str) -> bool {
        let norm = Self::normalize_path(path);
        if self.entries.contains_key(&norm) {
            return false;
        }
        let parent = Self::parent_path(&norm);
        if let Some(FsEntry::Directory { children }) = self.entries.get_mut(&parent) {
            children.push(Self::file_name(&norm));
            self.entries.insert(norm, FsEntry::Directory { children: Vec::new() });
            true
        } else {
            false
        }
    }

    pub fn write_file(&mut self, path: &str, content: &[u8]) -> bool {
        let norm = Self::normalize_path(path);
        let parent = Self::parent_path(&norm);
        if !self.entries.contains_key(&parent) {
            return false;
        }
        let name = Self::file_name(&norm);
        if !self.entries.contains_key(&norm) {
            if let Some(FsEntry::Directory { children }) = self.entries.get_mut(&parent) {
                children.push(name);
            }
        }
        let size = content.len();
        self.entries.insert(norm, FsEntry::File { content: content.to_vec(), size });
        true
    }

    pub fn read_file(&self, path: &str) -> Option<&[u8]> {
        let norm = Self::normalize_path(path);
        match self.entries.get(&norm) {
            Some(FsEntry::File { content, .. }) => Some(content),
            _ => None,
        }
    }

    pub fn exists(&self, path: &str) -> bool {
        self.entries.contains_key(&Self::normalize_path(path))
    }

    pub fn is_dir(&self, path: &str) -> bool {
        matches!(self.entries.get(&Self::normalize_path(path)), Some(FsEntry::Directory { .. }))
    }

    pub fn delete(&mut self, path: &str) -> bool {
        let norm = Self::normalize_path(path);
        if norm == "/" {
            return false;
        }
        if self.entries.remove(&norm).is_some() {
            let parent = Self::parent_path(&norm);
            let name = Self::file_name(&norm);
            if let Some(FsEntry::Directory { children }) = self.entries.get_mut(&parent) {
                children.retain(|c| c != &name);
            }
            true
        } else {
            false
        }
    }

    pub fn list_dir(&self, path: &str) -> Vec<String> {
        let norm = Self::normalize_path(path);
        match self.entries.get(&norm) {
            Some(FsEntry::Directory { children }) => children.clone(),
            _ => Vec::new(),
        }
    }

    pub fn file_size(&self, path: &str) -> i64 {
        let norm = Self::normalize_path(path);
        match self.entries.get(&norm) {
            Some(FsEntry::File { size, .. }) => *size as i64,
            _ => -1,
        }
    }

    pub fn rename(&mut self, from: &str, to: &str) -> bool {
        let from_norm = Self::normalize_path(from);
        let to_norm = Self::normalize_path(to);
        if let Some(entry) = self.entries.remove(&from_norm) {
            let old_parent = Self::parent_path(&from_norm);
            let old_name = Self::file_name(&from_norm);
            if let Some(FsEntry::Directory { children }) = self.entries.get_mut(&old_parent) {
                children.retain(|c| c != &old_name);
            }
            let new_parent = Self::parent_path(&to_norm);
            let new_name = Self::file_name(&to_norm);
            if let Some(FsEntry::Directory { children }) = self.entries.get_mut(&new_parent) {
                children.push(new_name);
            }
            self.entries.insert(to_norm, entry);
            true
        } else {
            false
        }
    }

    pub fn entry_count(&self) -> i64 {
        self.entries.len() as i64
    }
}

use std::sync::Mutex;
use std::sync::LazyLock;
static VFS: LazyLock<Mutex<VirtualFs>> = LazyLock::new(|| Mutex::new(VirtualFs::new()));

#[unsafe(no_mangle)]
pub extern "C" fn slang_vfs_create() -> i64 {
    *VFS.lock().unwrap() = VirtualFs::new();
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_vfs_write(path_hash: i64, size: i64) -> i64 {
    let mut vfs = VFS.lock().unwrap();
    let path = format!("/file_{}", path_hash);
    let content = vec![0u8; size.max(0) as usize];
    if vfs.write_file(&path, &content) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_vfs_read(path_hash: i64) -> i64 {
    let vfs = VFS.lock().unwrap();
    let path = format!("/file_{}", path_hash);
    if vfs.read_file(&path).is_some() { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_vfs_exists(path_hash: i64) -> i64 {
    let vfs = VFS.lock().unwrap();
    let path = format!("/file_{}", path_hash);
    if vfs.exists(&path) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_vfs_delete(path_hash: i64) -> i64 {
    let mut vfs = VFS.lock().unwrap();
    let path = format!("/file_{}", path_hash);
    if vfs.delete(&path) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_vfs_list(path_hash: i64) -> i64 {
    let vfs = VFS.lock().unwrap();
    let path = format!("/dir_{}", path_hash);
    vfs.list_dir(&path).len() as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_vfs_mkdir(path_hash: i64) -> i64 {
    let mut vfs = VFS.lock().unwrap();
    let path = format!("/dir_{}", path_hash);
    if vfs.mkdir(&path) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_vfs_size(path_hash: i64) -> i64 {
    let vfs = VFS.lock().unwrap();
    let path = format!("/file_{}", path_hash);
    vfs.file_size(&path)
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_vfs_is_dir(path_hash: i64) -> i64 {
    let vfs = VFS.lock().unwrap();
    let path = format!("/dir_{}", path_hash);
    if vfs.is_dir(&path) { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn slang_vfs_rename(from_hash: i64, to_hash: i64) -> i64 {
    let mut vfs = VFS.lock().unwrap();
    let from = format!("/file_{}", from_hash);
    let to = format!("/file_{}", to_hash);
    if vfs.rename(&from, &to) { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_vfs() {
        let vfs = VirtualFs::new();
        assert!(vfs.exists("/"));
        assert!(vfs.is_dir("/"));
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(VirtualFs::normalize_path("/a/b/c"), "/a/b/c");
        assert_eq!(VirtualFs::normalize_path("/a/../b"), "/b");
        assert_eq!(VirtualFs::normalize_path("/a/./b"), "/a/b");
        assert_eq!(VirtualFs::normalize_path("///a///b"), "/a/b");
    }

    #[test]
    fn test_parent_path() {
        assert_eq!(VirtualFs::parent_path("/a/b"), "/a");
        assert_eq!(VirtualFs::parent_path("/a"), "/");
        assert_eq!(VirtualFs::parent_path("/"), "/");
    }

    #[test]
    fn test_file_name() {
        assert_eq!(VirtualFs::file_name("/a/b/c.txt"), "c.txt");
        assert_eq!(VirtualFs::file_name("/hello"), "hello");
    }

    #[test]
    fn test_mkdir() {
        let mut vfs = VirtualFs::new();
        assert!(vfs.mkdir("/docs"));
        assert!(vfs.is_dir("/docs"));
        assert!(vfs.exists("/docs"));
    }

    #[test]
    fn test_mkdir_duplicate() {
        let mut vfs = VirtualFs::new();
        assert!(vfs.mkdir("/docs"));
        assert!(!vfs.mkdir("/docs"));
    }

    #[test]
    fn test_write_read_file() {
        let mut vfs = VirtualFs::new();
        assert!(vfs.write_file("/test.txt", b"hello world"));
        let content = vfs.read_file("/test.txt").unwrap();
        assert_eq!(content, b"hello world");
    }

    #[test]
    fn test_file_size() {
        let mut vfs = VirtualFs::new();
        vfs.write_file("/data.bin", &[1, 2, 3, 4, 5]);
        assert_eq!(vfs.file_size("/data.bin"), 5);
    }

    #[test]
    fn test_file_size_nonexistent() {
        let vfs = VirtualFs::new();
        assert_eq!(vfs.file_size("/nope"), -1);
    }

    #[test]
    fn test_delete_file() {
        let mut vfs = VirtualFs::new();
        vfs.write_file("/x.txt", b"data");
        assert!(vfs.delete("/x.txt"));
        assert!(!vfs.exists("/x.txt"));
    }

    #[test]
    fn test_delete_nonexistent() {
        let mut vfs = VirtualFs::new();
        assert!(!vfs.delete("/nope"));
    }

    #[test]
    fn test_cannot_delete_root() {
        let mut vfs = VirtualFs::new();
        assert!(!vfs.delete("/"));
    }

    #[test]
    fn test_list_dir() {
        let mut vfs = VirtualFs::new();
        vfs.write_file("/a.txt", b"a");
        vfs.write_file("/b.txt", b"b");
        let listing = vfs.list_dir("/");
        assert!(listing.contains(&"a.txt".to_string()));
        assert!(listing.contains(&"b.txt".to_string()));
    }

    #[test]
    fn test_list_empty_dir() {
        let mut vfs = VirtualFs::new();
        vfs.mkdir("/empty");
        assert!(vfs.list_dir("/empty").is_empty());
    }

    #[test]
    fn test_nested_dirs() {
        let mut vfs = VirtualFs::new();
        assert!(vfs.mkdir("/a"));
        assert!(vfs.mkdir("/a/b"));
        assert!(vfs.is_dir("/a/b"));
    }

    #[test]
    fn test_rename() {
        let mut vfs = VirtualFs::new();
        vfs.write_file("/old.txt", b"content");
        assert!(vfs.rename("/old.txt", "/new.txt"));
        assert!(!vfs.exists("/old.txt"));
        assert!(vfs.exists("/new.txt"));
    }

    #[test]
    fn test_rename_nonexistent() {
        let mut vfs = VirtualFs::new();
        assert!(!vfs.rename("/nope", "/also_nope"));
    }

    #[test]
    fn test_overwrite_file() {
        let mut vfs = VirtualFs::new();
        vfs.write_file("/f.txt", b"old");
        vfs.write_file("/f.txt", b"new content");
        let content = vfs.read_file("/f.txt").unwrap();
        assert_eq!(content, b"new content");
    }

    #[test]
    fn test_read_nonexistent() {
        let vfs = VirtualFs::new();
        assert!(vfs.read_file("/nope").is_none());
    }

    #[test]
    fn test_is_dir_on_file() {
        let mut vfs = VirtualFs::new();
        vfs.write_file("/f.txt", b"data");
        assert!(!vfs.is_dir("/f.txt"));
    }

    #[test]
    fn test_entry_count() {
        let mut vfs = VirtualFs::new();
        assert_eq!(vfs.entry_count(), 1); // root
        vfs.write_file("/a", b"");
        vfs.write_file("/b", b"");
        assert_eq!(vfs.entry_count(), 3);
    }

    #[test]
    fn test_empty_file() {
        let mut vfs = VirtualFs::new();
        vfs.write_file("/empty", b"");
        assert_eq!(vfs.file_size("/empty"), 0);
    }
}

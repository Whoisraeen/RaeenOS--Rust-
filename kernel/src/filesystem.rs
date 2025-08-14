use alloc::collections::BTreeMap;
use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::boxed::Box;
use core::fmt;
use spin::{Mutex, RwLock};

static VFS: RwLock<VirtualFileSystem> = RwLock::new(VirtualFileSystem::new());
static NEXT_FD: Mutex<u64> = Mutex::new(3); // Start after stdin, stdout, stderr

// Public handle type used by other subsystems
pub type FileHandle = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Regular,
    Directory,
    SymbolicLink,
    CharacterDevice,
    BlockDevice,
    Fifo,
    Socket,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekFrom {
    Start(u64),
    End(i64),
    Current(i64),
}

#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub file_type: FileType,
    pub size: u64,
    pub permissions: u32,
    pub created: u64,
    pub modified: u64,
    pub accessed: u64,
    pub uid: u32,
    pub gid: u32,
}

impl Default for FileMetadata {
    fn default() -> Self {
        let now = crate::time::get_timestamp();
        Self {
            file_type: FileType::Regular,
            size: 0,
            permissions: 0o644,
            created: now,
            modified: now,
            accessed: now,
            uid: 0,
            gid: 0,
        }
    }
}

#[derive(Debug)]
pub enum FileSystemError {
    NotFound,
    PermissionDenied,
    AlreadyExists,
    NotADirectory,
    IsADirectory,
    InvalidPath,
    IoError,
    NoSpace,
    ReadOnly,
    InvalidOperation,
}

impl fmt::Display for FileSystemError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FileSystemError::NotFound => write!(f, "File or directory not found"),
            FileSystemError::PermissionDenied => write!(f, "Permission denied"),
            FileSystemError::AlreadyExists => write!(f, "File or directory already exists"),
            FileSystemError::NotADirectory => write!(f, "Not a directory"),
            FileSystemError::IsADirectory => write!(f, "Is a directory"),
            FileSystemError::InvalidPath => write!(f, "Invalid path"),
            FileSystemError::IoError => write!(f, "I/O error"),
            FileSystemError::NoSpace => write!(f, "No space left on device"),
            FileSystemError::ReadOnly => write!(f, "Read-only file system"),
            FileSystemError::InvalidOperation => write!(f, "Invalid operation"),
        }
    }
}

pub type FileSystemResult<T> = Result<T, FileSystemError>;

// Virtual File System trait
pub trait FileSystem: Send + Sync {
    fn name(&self) -> &str;
    fn open(&mut self, path: &str, flags: u32) -> FileSystemResult<Box<dyn File>>;
    fn create(&mut self, path: &str, file_type: FileType) -> FileSystemResult<()>;
    fn remove(&mut self, path: &str) -> FileSystemResult<()>;
    fn metadata(&self, path: &str) -> FileSystemResult<FileMetadata>;
    fn list_directory(&self, path: &str) -> FileSystemResult<Vec<String>>;
    fn rename(&mut self, old_path: &str, new_path: &str) -> FileSystemResult<()>;
    fn sync(&mut self) -> FileSystemResult<()>;
}

// File trait for file operations
pub trait File: Send + Sync {
    fn read(&mut self, buffer: &mut [u8]) -> FileSystemResult<usize>;
    fn write(&mut self, buffer: &[u8]) -> FileSystemResult<usize>;
    fn seek(&mut self, pos: SeekFrom) -> FileSystemResult<u64>;
    fn flush(&mut self) -> FileSystemResult<()>;
    fn metadata(&self) -> FileSystemResult<FileMetadata>;
    fn set_permissions(&mut self, permissions: u32) -> FileSystemResult<()>;
}

// In-memory file system implementation
#[derive(Debug)]
pub struct MemoryFileSystem {
    name: String,
    root: MemoryNode,
    next_inode: u64,
}

#[derive(Debug, Clone)]
struct MemoryNode {
    inode: u64,
    name: String,
    metadata: FileMetadata,
    data: Vec<u8>,
    children: BTreeMap<String, MemoryNode>,
}

impl MemoryNode {
    fn new(name: String, file_type: FileType, inode: u64) -> Self {
        let mut metadata = FileMetadata::default();
        metadata.file_type = file_type;
        
        Self {
            inode,
            name,
            metadata,
            data: Vec::new(),
            children: BTreeMap::new(),
        }
    }
    
    fn find_node(&self, path: &str) -> Option<&MemoryNode> {
        if path.is_empty() || path == "/" {
            return Some(self);
        }
        
        let path = path.trim_start_matches('/');
        let parts: Vec<&str> = path.split('/').collect();
        
        let mut current = self;
        for part in parts {
            if part.is_empty() {
                continue;
            }
            current = current.children.get(part)?;
        }
        
        Some(current)
    }
    
    fn find_node_mut(&mut self, path: &str) -> Option<&mut MemoryNode> {
        if path.is_empty() || path == "/" {
            return Some(self);
        }
        
        let path = path.trim_start_matches('/');
        let parts: Vec<&str> = path.split('/').collect();
        
        let mut current = self;
        for part in parts {
            if part.is_empty() {
                continue;
            }
            current = current.children.get_mut(part)?;
        }
        
        Some(current)
    }
    
    fn create_node(&mut self, path: &str, file_type: FileType, inode: u64) -> FileSystemResult<()> {
        let path = path.trim_start_matches('/');
        let parts: Vec<&str> = path.split('/').collect();
        
        if parts.is_empty() {
            return Err(FileSystemError::InvalidPath);
        }
        
        let (parent_parts, file_name_arr) = parts.split_at(parts.len() - 1);
        let file_name = file_name_arr[0];
        
        // Navigate to parent directory
        let mut current = self;
        for part in parent_parts {
            if part.is_empty() {
                continue;
            }
            current = current.children.get_mut(*part)
                .ok_or(FileSystemError::NotFound)?;
            
            if current.metadata.file_type != FileType::Directory {
                return Err(FileSystemError::NotADirectory);
            }
        }
        
        // Check if file already exists
        if current.children.contains_key(file_name) {
            return Err(FileSystemError::AlreadyExists);
        }
        
        // Create new node
        let new_node = MemoryNode::new(file_name.to_owned(), file_type, inode);
        current.children.insert(file_name.to_owned(), new_node);
        
        Ok(())
    }
}

impl MemoryFileSystem {
    pub fn new(name: String) -> Self {
        let root = MemoryNode::new("/".to_owned(), FileType::Directory, 1);
        Self {
            name,
            root,
            next_inode: 2,
        }
    }
    
    fn allocate_inode(&mut self) -> u64 {
        let inode = self.next_inode;
        self.next_inode += 1;
        inode
    }
}

impl FileSystem for MemoryFileSystem {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn open(&mut self, path: &str, _flags: u32) -> FileSystemResult<Box<dyn File>> {
        let node = self.root.find_node(path)
            .ok_or(FileSystemError::NotFound)?;
        
        if node.metadata.file_type == FileType::Directory {
            return Err(FileSystemError::IsADirectory);
        }
        
        Ok(Box::new(MemoryFile {
            data: node.data.clone(),
            metadata: node.metadata.clone(),
            position: 0,
        }))
    }
    
    fn create(&mut self, path: &str, file_type: FileType) -> FileSystemResult<()> {
        let inode = self.allocate_inode();
        self.root.create_node(path, file_type, inode)
    }
    
    fn remove(&mut self, path: &str) -> FileSystemResult<()> {
        if path == "/" {
            return Err(FileSystemError::InvalidOperation);
        }
        
        let path = path.trim_start_matches('/');
        let parts: Vec<&str> = path.split('/').collect();
        
        if parts.is_empty() {
            return Err(FileSystemError::InvalidPath);
        }
        
        let (parent_parts, file_name_arr) = parts.split_at(parts.len() - 1);
        let file_name = file_name_arr[0];
        
        // Navigate to parent directory
        let mut current = &mut self.root;
        for part in parent_parts {
            if part.is_empty() {
                continue;
            }
            current = current.children.get_mut(*part)
                .ok_or(FileSystemError::NotFound)?;
            
            if current.metadata.file_type != FileType::Directory {
                return Err(FileSystemError::NotADirectory);
            }
        }
        
        // Check if file exists
        if !current.children.contains_key(file_name) {
            return Err(FileSystemError::NotFound);
        }
        
        // Remove the file/directory
        current.children.remove(file_name);
        Ok(())
    }
    
    fn metadata(&self, path: &str) -> FileSystemResult<FileMetadata> {
        let node = self.root.find_node(path)
            .ok_or(FileSystemError::NotFound)?;
        Ok(node.metadata.clone())
    }
    
    fn list_directory(&self, path: &str) -> FileSystemResult<Vec<String>> {
        let node = self.root.find_node(path)
            .ok_or(FileSystemError::NotFound)?;
        
        if node.metadata.file_type != FileType::Directory {
            return Err(FileSystemError::NotADirectory);
        }
        
        Ok(node.children.keys().cloned().collect())
    }
    
    fn rename(&mut self, old_path: &str, new_path: &str) -> FileSystemResult<()> {
        if old_path == "/" || new_path == "/" {
            return Err(FileSystemError::InvalidOperation);
        }
        
        // First, check if the old path exists and get the node
        let old_node = {
            let node = self.root.find_node(old_path)
                .ok_or(FileSystemError::NotFound)?;
            node.clone()
        };
        
        // Check if new path already exists
        if self.root.find_node(new_path).is_some() {
            return Err(FileSystemError::AlreadyExists);
        }
        
        // Create the new node at the new path
        let new_path_trimmed = new_path.trim_start_matches('/');
        let new_parts: Vec<&str> = new_path_trimmed.split('/').collect();
        
        if new_parts.is_empty() {
            return Err(FileSystemError::InvalidPath);
        }
        
        let (new_parent_parts, new_file_name_arr) = new_parts.split_at(new_parts.len() - 1);
        let new_file_name = new_file_name_arr[0];
        
        // Navigate to new parent directory
        let mut new_parent = &mut self.root;
        for part in new_parent_parts {
            if part.is_empty() {
                continue;
            }
            new_parent = new_parent.children.get_mut(*part)
                .ok_or(FileSystemError::NotFound)?;
            
            if new_parent.metadata.file_type != FileType::Directory {
                return Err(FileSystemError::NotADirectory);
            }
        }
        
        // Create new node with updated name
        let mut new_node = old_node;
        new_node.name = new_file_name.to_owned();
        new_parent.children.insert(new_file_name.to_owned(), new_node);
        
        // Remove the old node
        self.remove(old_path)?;
        
        Ok(())
    }
    
    fn sync(&mut self) -> FileSystemResult<()> {
        // Memory filesystem doesn't need syncing
        Ok(())
    }
}

#[derive(Debug)]
struct MemoryFile {
    data: Vec<u8>,
    metadata: FileMetadata,
    position: u64,
}

impl File for MemoryFile {
    fn read(&mut self, buffer: &mut [u8]) -> FileSystemResult<usize> {
        let start = self.position as usize;
        let end = core::cmp::min(start + buffer.len(), self.data.len());
        
        if start >= self.data.len() {
            return Ok(0);
        }
        
        let bytes_read = end - start;
        buffer[..bytes_read].copy_from_slice(&self.data[start..end]);
        self.position += bytes_read as u64;
        
        Ok(bytes_read)
    }
    
    fn write(&mut self, buffer: &[u8]) -> FileSystemResult<usize> {
        let start = self.position as usize;
        let end = start + buffer.len();
        
        // Extend data if necessary
        if end > self.data.len() {
            self.data.resize(end, 0);
        }
        
        self.data[start..end].copy_from_slice(buffer);
        self.position += buffer.len() as u64;
        self.metadata.size = self.data.len() as u64;
        self.metadata.modified = crate::time::get_timestamp();
        
        Ok(buffer.len())
    }
    
    fn seek(&mut self, pos: SeekFrom) -> FileSystemResult<u64> {
        let new_pos = match pos {
            SeekFrom::Start(offset) => offset,
            SeekFrom::End(offset) => {
                if offset < 0 && (-offset) as u64 > self.data.len() as u64 {
                    0
                } else {
                    (self.data.len() as i64 + offset) as u64
                }
            }
            SeekFrom::Current(offset) => {
                if offset < 0 && (-offset) as u64 > self.position {
                    0
                } else {
                    (self.position as i64 + offset) as u64
                }
            }
        };
        
        self.position = new_pos;
        Ok(self.position)
    }
    
    fn flush(&mut self) -> FileSystemResult<()> {
        // Memory file doesn't need flushing
        Ok(())
    }
    
    fn metadata(&self) -> FileSystemResult<FileMetadata> {
        Ok(self.metadata.clone())
    }
    
    fn set_permissions(&mut self, permissions: u32) -> FileSystemResult<()> {
        self.metadata.permissions = permissions;
        Ok(())
    }
}

// Virtual File System
pub struct VirtualFileSystem {
    filesystems: BTreeMap<String, Box<dyn FileSystem>>,
    mount_points: BTreeMap<String, String>, // mount_point -> filesystem_name
    open_files: BTreeMap<u64, Box<dyn File>>, // fd -> file
}

impl VirtualFileSystem {
    pub const fn new() -> Self {
        Self {
            filesystems: BTreeMap::new(),
            mount_points: BTreeMap::new(),
            open_files: BTreeMap::new(),
        }
    }
    
    pub fn mount(&mut self, filesystem: Box<dyn FileSystem>, mount_point: &str) -> FileSystemResult<()> {
        let fs_name = filesystem.name().to_owned();
        self.filesystems.insert(fs_name.clone(), filesystem);
        self.mount_points.insert(mount_point.to_owned(), fs_name);
        Ok(())
    }
    
    pub fn unmount(&mut self, mount_point: &str) -> FileSystemResult<()> {
        if let Some(fs_name) = self.mount_points.remove(mount_point) {
            self.filesystems.remove(&fs_name);
            Ok(())
        } else {
            Err(FileSystemError::NotFound)
        }
    }
    
    fn resolve_path(&self, path: &str) -> Option<(String, String)> {
        // Find the longest matching mount point
        let mut best_match = ("/".to_owned(), "");
        let mut best_len = 0;
        
        for (mount_point, fs_name) in &self.mount_points {
            if path.starts_with(mount_point) && mount_point.len() > best_len {
                best_match = (mount_point.clone(), fs_name.as_str());
                best_len = mount_point.len();
            }
        }
        
        if best_len > 0 {
            Some((best_match.0, path[best_len..].to_owned()))
        } else {
            None
        }
    }
    
    pub fn open(&mut self, path: &str, flags: u32) -> FileSystemResult<u64> {
        let (fs_name, relative_path) = self.resolve_path(path)
            .ok_or(FileSystemError::NotFound)?;
        
        let filesystem = self.filesystems.get_mut(&fs_name)
            .ok_or(FileSystemError::NotFound)?;
        
        let file = filesystem.open(&relative_path, flags)?;
        let fd = *NEXT_FD.lock();
        *NEXT_FD.lock() += 1;
        
        self.open_files.insert(fd, file);
        Ok(fd)
    }
    
    pub fn close(&mut self, fd: u64) -> FileSystemResult<()> {
        self.open_files.remove(&fd)
            .ok_or(FileSystemError::NotFound)?;
        Ok(())
    }
    
    pub fn read(&mut self, fd: u64, buffer: &mut [u8]) -> FileSystemResult<usize> {
        let file = self.open_files.get_mut(&fd)
            .ok_or(FileSystemError::NotFound)?;
        file.read(buffer)
    }
    
    pub fn write(&mut self, fd: u64, buffer: &[u8]) -> FileSystemResult<usize> {
        let file = self.open_files.get_mut(&fd)
            .ok_or(FileSystemError::NotFound)?;
        file.write(buffer)
    }
    
    pub fn seek(&mut self, fd: u64, pos: SeekFrom) -> FileSystemResult<u64> {
        let file = self.open_files.get_mut(&fd)
            .ok_or(FileSystemError::NotFound)?;
        file.seek(pos)
    }
    
    pub fn create(&mut self, path: &str, file_type: FileType) -> FileSystemResult<()> {
        let (fs_name, relative_path) = self.resolve_path(path)
            .ok_or(FileSystemError::NotFound)?;
        
        let filesystem = self.filesystems.get_mut(&fs_name)
            .ok_or(FileSystemError::NotFound)?;
        
        filesystem.create(&relative_path, file_type)
    }
    
    pub fn remove(&mut self, path: &str) -> FileSystemResult<()> {
        let (fs_name, relative_path) = self.resolve_path(path)
            .ok_or(FileSystemError::NotFound)?;
        
        let filesystem = self.filesystems.get_mut(&fs_name)
            .ok_or(FileSystemError::NotFound)?;
        
        filesystem.remove(&relative_path)
    }
    
    pub fn metadata(&self, path: &str) -> FileSystemResult<FileMetadata> {
        let (fs_name, relative_path) = self.resolve_path(path)
            .ok_or(FileSystemError::NotFound)?;
        
        let filesystem = self.filesystems.get(&fs_name)
            .ok_or(FileSystemError::NotFound)?;
        
        filesystem.metadata(&relative_path)
    }
    
    pub fn list_directory(&self, path: &str) -> FileSystemResult<Vec<String>> {
        let (fs_name, relative_path) = self.resolve_path(path)
            .ok_or(FileSystemError::NotFound)?;
        
        let filesystem = self.filesystems.get(&fs_name)
            .ok_or(FileSystemError::NotFound)?;
        
        filesystem.list_directory(&relative_path)
    }
}

// Public API functions
pub fn init() {
    let mut vfs = VFS.write();
    
    // Create and mount root filesystem
    let root_fs = Box::new(MemoryFileSystem::new("rootfs".to_owned()));
    vfs.mount(root_fs, "/").expect("Failed to mount root filesystem");
    
    // Create standard directories
    let _ = vfs.create("/bin", FileType::Directory);
    let _ = vfs.create("/etc", FileType::Directory);
    let _ = vfs.create("/home", FileType::Directory);
    let _ = vfs.create("/tmp", FileType::Directory);
    let _ = vfs.create("/var", FileType::Directory);
    let _ = vfs.create("/dev", FileType::Directory);
    let _ = vfs.create("/proc", FileType::Directory);
    let _ = vfs.create("/sys", FileType::Directory);
}

pub fn open(path: &str, flags: u32) -> FileSystemResult<u64> {
    VFS.write().open(path, flags)
}

pub fn close(fd: u64) -> FileSystemResult<()> {
    VFS.write().close(fd)
}

pub fn read(fd: u64, buffer: &mut [u8]) -> FileSystemResult<usize> {
    VFS.write().read(fd, buffer)
}

pub fn write(fd: u64, buffer: &[u8]) -> FileSystemResult<usize> {
    VFS.write().write(fd, buffer)
}

pub fn seek(fd: u64, pos: SeekFrom) -> FileSystemResult<u64> {
    VFS.write().seek(fd, pos)
}

pub fn create_file(path: &str) -> FileSystemResult<()> {
    VFS.write().create(path, FileType::Regular)
}

pub fn create_directory(path: &str) -> FileSystemResult<()> {
    VFS.write().create(path, FileType::Directory)
}

pub fn remove(path: &str) -> FileSystemResult<()> {
    VFS.write().remove(path)
}

pub fn metadata(path: &str) -> FileSystemResult<FileMetadata> {
    VFS.read().metadata(path)
}

pub fn list_directory(path: &str) -> FileSystemResult<Vec<String>> {
    VFS.read().list_directory(path)
}

pub fn mount_filesystem(filesystem: Box<dyn FileSystem>, mount_point: &str) -> FileSystemResult<()> {
    VFS.write().mount(filesystem, mount_point)
}

pub fn unmount_filesystem(mount_point: &str) -> FileSystemResult<()> {
    VFS.write().unmount(mount_point)
}
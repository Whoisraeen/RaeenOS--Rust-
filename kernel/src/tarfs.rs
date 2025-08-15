//! TAR filesystem implementation for RaeenOS
//! 
//! This module provides a simple read-only TAR filesystem that can be used
//! to load embedded archives or preloaded binaries into the VFS.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::borrow::ToOwned;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::format;
use core::str;

use crate::filesystem::{
    FileSystem, File, FileType, FileMetadata, FileSystemError, FileSystemResult
};

/// TAR header structure (POSIX.1-1988 format)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct TarHeader {
    name: [u8; 100],
    mode: [u8; 8],
    uid: [u8; 8],
    gid: [u8; 8],
    size: [u8; 12],
    mtime: [u8; 12],
    checksum: [u8; 8],
    typeflag: u8,
    linkname: [u8; 100],
    magic: [u8; 6],
    version: [u8; 2],
    uname: [u8; 32],
    gname: [u8; 32],
    devmajor: [u8; 8],
    devminor: [u8; 8],
    prefix: [u8; 155],
    _padding: [u8; 12],
}

const TAR_BLOCK_SIZE: usize = 512;
const TAR_MAGIC: &[u8] = b"ustar";

/// TAR file type flags
const TAR_TYPE_REGULAR: u8 = b'0';
const TAR_TYPE_DIRECTORY: u8 = b'5';

/// TAR file entry
#[derive(Debug, Clone)]
struct TarEntry {
    name: String,
    file_type: FileType,
    size: u64,
    data_offset: usize,
    metadata: FileMetadata,
}

/// TAR filesystem implementation
pub struct TarFileSystem {
    name: String,
    data: Vec<u8>,
    entries: BTreeMap<String, TarEntry>,
}

impl TarFileSystem {
    /// Create a new TAR filesystem from raw data
    pub fn new(name: String, data: Vec<u8>) -> Result<Self, FileSystemError> {
        let mut tarfs = Self {
            name,
            data,
            entries: BTreeMap::new(),
        };
        
        tarfs.parse_archive()?;
        Ok(tarfs)
    }
    
    /// Parse the TAR archive and build the file index
    fn parse_archive(&mut self) -> Result<(), FileSystemError> {
        let mut offset = 0;
        
        while offset + TAR_BLOCK_SIZE <= self.data.len() {
            // Check if we've reached the end (two zero blocks)
            if self.is_zero_block(offset) && 
               offset + TAR_BLOCK_SIZE < self.data.len() &&
               self.is_zero_block(offset + TAR_BLOCK_SIZE) {
                break;
            }
            
            let header = self.parse_header(offset)?;
            if let Some((entry, data_size)) = header {
                self.entries.insert(entry.name.clone(), entry);
                
                // Move to next header (data is padded to 512-byte boundary)
                let padded_size = (data_size + TAR_BLOCK_SIZE - 1) & !(TAR_BLOCK_SIZE - 1);
                offset += TAR_BLOCK_SIZE + padded_size;
            } else {
                break;
            }
        }
        
        Ok(())
    }
    
    /// Check if a block is all zeros
    fn is_zero_block(&self, offset: usize) -> bool {
        if offset + TAR_BLOCK_SIZE > self.data.len() {
            return false;
        }
        
        self.data[offset..offset + TAR_BLOCK_SIZE].iter().all(|&b| b == 0)
    }
    
    /// Parse a TAR header at the given offset
    fn parse_header(&self, offset: usize) -> Result<Option<(TarEntry, usize)>, FileSystemError> {
        if offset + TAR_BLOCK_SIZE > self.data.len() {
            return Ok(None);
        }
        
        let header_bytes = &self.data[offset..offset + TAR_BLOCK_SIZE];
        let header = unsafe { &*(header_bytes.as_ptr() as *const TarHeader) };
        
        // Verify magic number
        if &header.magic[..5] != TAR_MAGIC {
            return Ok(None);
        }
        
        // Parse filename
        let name = self.parse_cstring(&header.name)?;
        if name.is_empty() {
            return Ok(None);
        }
        
        // Parse file size
        let size_str = self.parse_cstring(&header.size)?;
        let size = self.parse_octal(&size_str).unwrap_or(0);
        
        // Parse file type
        let file_type = match header.typeflag {
            TAR_TYPE_REGULAR | 0 => FileType::Regular,
            TAR_TYPE_DIRECTORY => FileType::Directory,
            _ => return Ok(None), // Skip unsupported types
        };
        
        // Parse mode
        let mode_str = self.parse_cstring(&header.mode)?;
        let mode = self.parse_octal(&mode_str).unwrap_or(0o644);
        
        // Create metadata
        let mut metadata = FileMetadata::default();
        metadata.file_type = file_type;
        metadata.size = size;
        metadata.permissions = mode as u32;
        
        let entry = TarEntry {
            name: name.clone(),
            file_type,
            size,
            data_offset: offset + TAR_BLOCK_SIZE,
            metadata,
        };
        
        Ok(Some((entry, size as usize)))
    }
    
    /// Parse a null-terminated string from a byte array
    fn parse_cstring(&self, bytes: &[u8]) -> Result<String, FileSystemError> {
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        str::from_utf8(&bytes[..end])
            .map(|s| s.to_string())
            .map_err(|_| FileSystemError::InvalidPath)
    }
    
    /// Parse an octal string to integer
    fn parse_octal(&self, s: &str) -> Option<u64> {
        let s = s.trim();
        if s.is_empty() {
            return Some(0);
        }
        
        let mut result = 0u64;
        for c in s.chars() {
            if c >= '0' && c <= '7' {
                result = result * 8 + (c as u64 - '0' as u64);
            } else {
                return None;
            }
        }
        Some(result)
    }
    
    /// Normalize path (remove leading slash, handle relative paths)
    fn normalize_path(&self, path: &str) -> String {
        let path = path.trim_start_matches('/');
        if path.is_empty() {
            ".".to_owned()
        } else {
            path.to_owned()
        }
    }
}

impl FileSystem for TarFileSystem {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn open(&mut self, path: &str, _flags: u32) -> FileSystemResult<Box<dyn File>> {
        let normalized_path = self.normalize_path(path);
        
        if let Some(entry) = self.entries.get(&normalized_path) {
            if entry.file_type == FileType::Directory {
                return Err(FileSystemError::IsADirectory);
            }
            
            let file_data = if entry.size > 0 {
                let start = entry.data_offset;
                let end = start + entry.size as usize;
                if end <= self.data.len() {
                    self.data[start..end].to_vec()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };
            
            Ok(Box::new(TarFile {
                data: file_data,
                position: 0,
                metadata: entry.metadata.clone(),
            }))
        } else {
            Err(FileSystemError::NotFound)
        }
    }
    
    fn create(&mut self, _path: &str, _file_type: FileType) -> FileSystemResult<()> {
        Err(FileSystemError::ReadOnly)
    }
    
    fn remove(&mut self, _path: &str) -> FileSystemResult<()> {
        Err(FileSystemError::ReadOnly)
    }
    
    fn metadata(&self, path: &str) -> FileSystemResult<FileMetadata> {
        let normalized_path = self.normalize_path(path);
        
        if let Some(entry) = self.entries.get(&normalized_path) {
            Ok(entry.metadata.clone())
        } else {
            Err(FileSystemError::NotFound)
        }
    }
    
    fn list_directory(&self, path: &str) -> FileSystemResult<Vec<String>> {
        let normalized_path = self.normalize_path(path);
        
        // Check if the path is a directory
        if let Some(entry) = self.entries.get(&normalized_path) {
            if entry.file_type != FileType::Directory {
                return Err(FileSystemError::NotADirectory);
            }
        } else if normalized_path != "." {
            return Err(FileSystemError::NotFound);
        }
        
        let mut children = Vec::new();
        let prefix = if normalized_path == "." {
            String::new()
        } else {
            format!("{}/", normalized_path)
        };
        
        for name in self.entries.keys() {
            if name.starts_with(&prefix) {
                let relative = &name[prefix.len()..];
                if !relative.is_empty() && !relative.contains('/') {
                    children.push(relative.to_owned());
                }
            }
        }
        
        Ok(children)
    }
    
    fn rename(&mut self, _old_path: &str, _new_path: &str) -> FileSystemResult<()> {
        Err(FileSystemError::ReadOnly)
    }
    
    fn sync(&mut self) -> FileSystemResult<()> {
        Ok(()) // Read-only filesystem, nothing to sync
    }
}

/// TAR file implementation
struct TarFile {
    data: Vec<u8>,
    position: usize,
    metadata: FileMetadata,
}

impl File for TarFile {
    fn read(&mut self, buffer: &mut [u8]) -> FileSystemResult<usize> {
        if self.position >= self.data.len() {
            return Ok(0);
        }
        
        let available = self.data.len() - self.position;
        let to_read = buffer.len().min(available);
        
        buffer[..to_read].copy_from_slice(&self.data[self.position..self.position + to_read]);
        self.position += to_read;
        
        Ok(to_read)
    }
    
    fn write(&mut self, _buffer: &[u8]) -> FileSystemResult<usize> {
        Err(FileSystemError::ReadOnly)
    }
    
    fn seek(&mut self, pos: crate::filesystem::SeekFrom) -> FileSystemResult<u64> {
        use crate::filesystem::SeekFrom;
        
        let new_pos = match pos {
            SeekFrom::Start(offset) => offset as usize,
            SeekFrom::End(offset) => {
                if offset >= 0 {
                    self.data.len() + offset as usize
                } else {
                    self.data.len().saturating_sub((-offset) as usize)
                }
            }
            SeekFrom::Current(offset) => {
                if offset >= 0 {
                    self.position + offset as usize
                } else {
                    self.position.saturating_sub((-offset) as usize)
                }
            }
        };
        
        self.position = new_pos.min(self.data.len());
        Ok(self.position as u64)
    }
    
    fn metadata(&self) -> FileSystemResult<FileMetadata> {
        Ok(self.metadata.clone())
    }
    
    fn flush(&mut self) -> FileSystemResult<()> {
        Ok(()) // Read-only file, nothing to flush
    }
    
    fn set_permissions(&mut self, _permissions: u32) -> FileSystemResult<()> {
        Err(FileSystemError::ReadOnly)
    }
}

/// Create a TAR filesystem from embedded data
pub fn create_tar_filesystem(name: String, data: Vec<u8>) -> Result<Box<dyn FileSystem>, FileSystemError> {
    let tarfs = TarFileSystem::new(name, data)?;
    Ok(Box::new(tarfs))
}

/// Load a simple embedded TAR archive for testing
pub fn create_test_tar_filesystem() -> Result<Box<dyn FileSystem>, FileSystemError> {
    // Create a minimal TAR archive with some test files
    let mut tar_data = Vec::new();
    
    // Add a simple test file "hello.txt"
    add_tar_file(&mut tar_data, "hello.txt", b"Hello, RaeenOS!\n");
    add_tar_file(&mut tar_data, "bin/test", b"#!/bin/sh\necho 'Test binary'\n");
    
    // Add end-of-archive marker (two zero blocks)
    tar_data.resize(tar_data.len() + TAR_BLOCK_SIZE * 2, 0);
    
    create_tar_filesystem("testfs".to_owned(), tar_data)
}

/// Helper function to add a file to TAR data
fn add_tar_file(tar_data: &mut Vec<u8>, name: &str, content: &[u8]) {
    let mut header = [0u8; TAR_BLOCK_SIZE];
    
    // Copy filename (truncate if too long)
    let name_bytes = name.as_bytes();
    let name_len = name_bytes.len().min(99);
    header[..name_len].copy_from_slice(&name_bytes[..name_len]);
    
    // Set mode (644 in octal)
    let mode = b"0000644";
    header[100..100 + mode.len()].copy_from_slice(mode);
    
    // Set size in octal
    let size_str = format!("{:011o}", content.len());
    let size_bytes = size_str.as_bytes();
    header[124..124 + size_bytes.len()].copy_from_slice(size_bytes);
    
    // Set file type (regular file)
    header[156] = TAR_TYPE_REGULAR;
    
    // Set magic and version
    header[257..262].copy_from_slice(TAR_MAGIC);
    header[263] = b'0';
    header[264] = b'0';
    
    // Calculate and set checksum
    let checksum = calculate_checksum(&header);
    let checksum_str = format!("{:06o}\0", checksum);
    let checksum_bytes = checksum_str.as_bytes();
    header[148..148 + checksum_bytes.len()].copy_from_slice(checksum_bytes);
    
    // Add header to TAR data
    tar_data.extend_from_slice(&header);
    
    // Add file content (padded to 512-byte boundary)
    tar_data.extend_from_slice(content);
    let padding = (TAR_BLOCK_SIZE - (content.len() % TAR_BLOCK_SIZE)) % TAR_BLOCK_SIZE;
    tar_data.resize(tar_data.len() + padding, 0);
}

/// Calculate TAR header checksum
fn calculate_checksum(header: &[u8; TAR_BLOCK_SIZE]) -> u32 {
    let mut sum = 0u32;
    
    for (i, &byte) in header.iter().enumerate() {
        if i >= 148 && i < 156 {
            // Checksum field is treated as spaces during calculation
            sum += b' ' as u32;
        } else {
            sum += byte as u32;
        }
    }
    
    sum
}
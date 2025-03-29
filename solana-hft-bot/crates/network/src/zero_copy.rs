//! Zero-copy buffer implementation
//! 
//! This module provides zero-copy buffer implementations for high-performance networking.

use std::alloc::{alloc, dealloc, Layout};
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;
use std::slice;

use anyhow::{anyhow, Result};

/// A buffer that can be used for zero-copy operations
pub struct ZeroCopyBuffer {
    /// Pointer to the buffer
    ptr: NonNull<u8>,
    
    /// Length of the buffer
    len: usize,
    
    /// Capacity of the buffer
    capacity: usize,
    
    /// Whether the buffer owns the memory
    owns_memory: bool,
}

impl ZeroCopyBuffer {
    /// Create a new zero-copy buffer with the given capacity
    pub fn new(capacity: usize) -> Result<Self> {
        // Align to page size for optimal performance
        let page_size = page_size::get();
        let aligned_capacity = (capacity + page_size - 1) & !(page_size - 1);
        
        // Allocate aligned memory
        let layout = Layout::from_size_align(aligned_capacity, page_size)
            .map_err(|e| anyhow!("Failed to create layout: {}", e))?;
        
        let ptr = unsafe { alloc(layout) };
        if ptr.is_null() {
            return Err(anyhow!("Memory allocation failed"));
        }
        
        Ok(Self {
            ptr: NonNull::new(ptr).unwrap(),
            len: 0,
            capacity: aligned_capacity,
            owns_memory: true,
        })
    }
    
    /// Create a zero-copy buffer from existing memory
    /// 
    /// # Safety
    /// 
    /// The provided pointer must be valid for reads and writes for `len` bytes,
    /// and must remain valid for the lifetime of the returned buffer.
    pub unsafe fn from_raw_parts(ptr: *mut u8, len: usize, capacity: usize) -> Self {
        Self {
            ptr: NonNull::new(ptr).unwrap(),
            len,
            capacity,
            owns_memory: false,
        }
    }
    
    /// Get the length of the buffer
    pub fn len(&self) -> usize {
        self.len
    }
    
    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    
    /// Get the capacity of the buffer
    pub fn capacity(&self) -> usize {
        self.capacity
    }
    
    /// Set the length of the buffer
    /// 
    /// # Safety
    /// 
    /// The caller must ensure that `new_len <= capacity` and that
    /// the memory is properly initialized up to `new_len`.
    pub unsafe fn set_len(&mut self, new_len: usize) {
        debug_assert!(new_len <= self.capacity);
        self.len = new_len;
    }
    
    /// Get a pointer to the buffer
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr.as_ptr()
    }
    
    /// Get a mutable pointer to the buffer
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr.as_ptr()
    }
    
    /// Copy data into the buffer
    pub fn copy_from_slice(&mut self, src: &[u8]) -> Result<()> {
        if src.len() > self.capacity {
            return Err(anyhow!("Source slice is too large for buffer"));
        }
        
        unsafe {
            std::ptr::copy_nonoverlapping(src.as_ptr(), self.ptr.as_ptr(), src.len());
            self.set_len(src.len());
        }
        
        Ok(())
    }
    
    /// Create a new buffer with the contents of the given slice
    pub fn from_slice(src: &[u8]) -> Result<Self> {
        let mut buffer = Self::new(src.len())?;
        buffer.copy_from_slice(src)?;
        Ok(buffer)
    }
    
    /// Resize the buffer
    pub fn resize(&mut self, new_capacity: usize) -> Result<()> {
        if new_capacity <= self.capacity {
            return Ok(());
        }
        
        // Align to page size
        let page_size = page_size::get();
        let aligned_capacity = (new_capacity + page_size - 1) & !(page_size - 1);
        
        // Allocate new memory
        let layout = Layout::from_size_align(aligned_capacity, page_size)
            .map_err(|e| anyhow!("Failed to create layout: {}", e))?;
        
        let new_ptr = unsafe { alloc(layout) };
        if new_ptr.is_null() {
            return Err(anyhow!("Memory allocation failed"));
        }
        
        // Copy data to new memory
        unsafe {
            std::ptr::copy_nonoverlapping(self.ptr.as_ptr(), new_ptr, self.len);
        }
        
        // Free old memory if owned
        if self.owns_memory {
            let old_layout = Layout::from_size_align(self.capacity, page_size)
                .expect("Invalid layout in resize");
            
            unsafe {
                dealloc(self.ptr.as_ptr(), old_layout);
            }
        }
        
        // Update buffer
        self.ptr = NonNull::new(new_ptr).unwrap();
        self.capacity = aligned_capacity;
        self.owns_memory = true;
        
        Ok(())
    }
}

impl Deref for ZeroCopyBuffer {
    type Target = [u8];
    
    fn deref(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl DerefMut for ZeroCopyBuffer {
    fn deref_mut(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}

impl Drop for ZeroCopyBuffer {
    fn drop(&mut self) {
        if self.owns_memory {
            let page_size = page_size::get();
            let layout = Layout::from_size_align(self.capacity, page_size)
                .expect("Invalid layout in drop");
            
            unsafe {
                dealloc(self.ptr.as_ptr(), layout);
            }
        }
    }
}

// Implement Send and Sync as we're managing the memory explicitly
unsafe impl Send for ZeroCopyBuffer {}
unsafe impl Sync for ZeroCopyBuffer {}

/// NUMA-aware zero-copy buffer
pub struct NumaZeroCopyBuffer {
    /// Inner buffer
    buffer: ZeroCopyBuffer,
    
    /// NUMA node
    numa_node: i32,
}

impl NumaZeroCopyBuffer {
    /// Create a new NUMA-aware zero-copy buffer
    pub fn new(capacity: usize, numa_node: i32) -> Result<Self> {
        #[cfg(target_os = "linux")]
        {
            use std::ptr::NonNull;
            
            // Align to page size
            let page_size = page_size::get();
            let aligned_capacity = (capacity + page_size - 1) & !(page_size - 1);
            
            // Allocate memory on the specified NUMA node
            let ptr = unsafe {
                let ptr = libc::numa_alloc_onnode(aligned_capacity, numa_node);
                if ptr.is_null() {
                    return Err(anyhow!("Failed to allocate memory on NUMA node {}", numa_node));
                }
                
                // Ensure the memory is page-aligned
                ptr as *mut u8
            };
            
            // Create buffer
            let buffer = unsafe {
                ZeroCopyBuffer {
                    ptr: NonNull::new(ptr).unwrap(),
                    len: 0,
                    capacity: aligned_capacity,
                    owns_memory: true,
                }
            };
            
            Ok(Self {
                buffer,
                numa_node,
            })
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            // Fallback to regular ZeroCopyBuffer on non-Linux systems
            let buffer = ZeroCopyBuffer::new(capacity)?;
            
            Ok(Self {
                buffer,
                numa_node: 0,
            })
        }
    }
    
    /// Get the NUMA node
    pub fn numa_node(&self) -> i32 {
        self.numa_node
    }
    
    /// Get a reference to the inner buffer
    pub fn inner(&self) -> &ZeroCopyBuffer {
        &self.buffer
    }
    
    /// Get a mutable reference to the inner buffer
    pub fn inner_mut(&mut self) -> &mut ZeroCopyBuffer {
        &mut self.buffer
    }
}

impl Deref for NumaZeroCopyBuffer {
    type Target = ZeroCopyBuffer;
    
    fn deref(&self) -> &ZeroCopyBuffer {
        &self.buffer
    }
}

impl DerefMut for NumaZeroCopyBuffer {
    fn deref_mut(&mut self) -> &mut ZeroCopyBuffer {
        &mut self.buffer
    }
}

impl Drop for NumaZeroCopyBuffer {
    fn drop(&mut self) {
        #[cfg(target_os = "linux")]
        {
            if self.buffer.owns_memory {
                unsafe {
                    libc::numa_free(
                        self.buffer.ptr.as_ptr() as *mut libc::c_void,
                        self.buffer.capacity,
                    );
                }
                
                // Prevent double-free in ZeroCopyBuffer's drop
                self.buffer.owns_memory = false;
            }
        }
    }
}

/// A ring buffer for zero-copy operations
pub struct ZeroCopyRingBuffer {
    /// Buffer
    buffer: ZeroCopyBuffer,
    
    /// Read position
    read_pos: usize,
    
    /// Write position
    write_pos: usize,
    
    /// Whether the buffer is full
    full: bool,
}

impl ZeroCopyRingBuffer {
    /// Create a new ring buffer with the given capacity
    pub fn new(capacity: usize) -> Result<Self> {
        let buffer = ZeroCopyBuffer::new(capacity)?;
        
        Ok(Self {
            buffer,
            read_pos: 0,
            write_pos: 0,
            full: false,
        })
    }
    
    /// Create a new NUMA-aware ring buffer
    pub fn new_numa(capacity: usize, numa_node: i32) -> Result<Self> {
        #[cfg(target_os = "linux")]
        {
            let numa_buffer = NumaZeroCopyBuffer::new(capacity, numa_node)?;
            let buffer = numa_buffer.inner().clone();
            
            Ok(Self {
                buffer,
                read_pos: 0,
                write_pos: 0,
                full: false,
            })
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            Self::new(capacity)
        }
    }
    
    /// Get the capacity of the ring buffer
    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }
    
    /// Get the number of bytes available to read
    pub fn available_read(&self) -> usize {
        if self.full {
            self.buffer.capacity()
        } else if self.write_pos >= self.read_pos {
            self.write_pos - self.read_pos
        } else {
            self.buffer.capacity() - (self.read_pos - self.write_pos)
        }
    }
    
    /// Get the number of bytes available to write
    pub fn available_write(&self) -> usize {
        self.buffer.capacity() - self.available_read()
    }
    
    /// Check if the ring buffer is empty
    pub fn is_empty(&self) -> bool {
        !self.full && self.read_pos == self.write_pos
    }
    
    /// Check if the ring buffer is full
    pub fn is_full(&self) -> bool {
        self.full
    }
    
    /// Write data to the ring buffer
    pub fn write(&mut self, src: &[u8]) -> Result<usize> {
        if self.is_full() {
            return Ok(0);
        }
        
        let available = self.available_write();
        let write_len = src.len().min(available);
        
        if write_len == 0 {
            return Ok(0);
        }
        
        // Handle wrap-around
        if self.write_pos + write_len <= self.buffer.capacity() {
            // No wrap-around
            unsafe {
                std::ptr::copy_nonoverlapping(
                    src.as_ptr(),
                    self.buffer.as_mut_ptr().add(self.write_pos),
                    write_len,
                );
            }
            
            self.write_pos += write_len;
            
            // Handle wrap-around for next write
            if self.write_pos == self.buffer.capacity() {
                self.write_pos = 0;
            }
        } else {
            // Wrap-around
            let first_chunk = self.buffer.capacity() - self.write_pos;
            let second_chunk = write_len - first_chunk;
            
            unsafe {
                // Write first chunk
                std::ptr::copy_nonoverlapping(
                    src.as_ptr(),
                    self.buffer.as_mut_ptr().add(self.write_pos),
                    first_chunk,
                );
                
                // Write second chunk
                std::ptr::copy_nonoverlapping(
                    src.as_ptr().add(first_chunk),
                    self.buffer.as_mut_ptr(),
                    second_chunk,
                );
            }
            
            self.write_pos = second_chunk;
        }
        
        // Check if buffer is now full
        if self.write_pos == self.read_pos {
            self.full = true;
        }
        
        Ok(write_len)
    }
    
    /// Read data from the ring buffer
    pub fn read(&mut self, dst: &mut [u8]) -> Result<usize> {
        if self.is_empty() {
            return Ok(0);
        }
        
        let available = self.available_read();
        let read_len = dst.len().min(available);
        
        if read_len == 0 {
            return Ok(0);
        }
        
        // Handle wrap-around
        if self.read_pos + read_len <= self.buffer.capacity() {
            // No wrap-around
            unsafe {
                std::ptr::copy_nonoverlapping(
                    self.buffer.as_ptr().add(self.read_pos),
                    dst.as_mut_ptr(),
                    read_len,
                );
            }
            
            self.read_pos += read_len;
            
            // Handle wrap-around for next read
            if self.read_pos == self.buffer.capacity() {
                self.read_pos = 0;
            }
        } else {
            // Wrap-around
            let first_chunk = self.buffer.capacity() - self.read_pos;
            let second_chunk = read_len - first_chunk;
            
            unsafe {
                // Read first chunk
                std::ptr::copy_nonoverlapping(
                    self.buffer.as_ptr().add(self.read_pos),
                    dst.as_mut_ptr(),
                    first_chunk,
                );
                
                // Read second chunk
                std::ptr::copy_nonoverlapping(
                    self.buffer.as_ptr(),
                    dst.as_mut_ptr().add(first_chunk),
                    second_chunk,
                );
            }
            
            self.read_pos = second_chunk;
        }
        
        // Buffer is no longer full
        self.full = false;
        
        Ok(read_len)
    }
    
    /// Clear the ring buffer
    pub fn clear(&mut self) {
        self.read_pos = 0;
        self.write_pos = 0;
        self.full = false;
    }
}

/// Memory pool for efficient buffer reuse
pub struct ZeroCopyMemoryPool {
    /// Buffers
    buffers: Vec<ZeroCopyBuffer>,
    
    /// Buffer size
    buffer_size: usize,
    
    /// Maximum number of buffers
    max_buffers: usize,
    
    /// NUMA node
    numa_node: Option<i32>,
}

impl ZeroCopyMemoryPool {
    /// Create a new memory pool
    pub fn new(buffer_size: usize, max_buffers: usize) -> Result<Self> {
        Ok(Self {
            buffers: Vec::with_capacity(max_buffers),
            buffer_size,
            max_buffers,
            numa_node: None,
        })
    }
    
    /// Create a new NUMA-aware memory pool
    pub fn new_numa(buffer_size: usize, max_buffers: usize, numa_node: i32) -> Result<Self> {
        Ok(Self {
            buffers: Vec::with_capacity(max_buffers),
            buffer_size,
            max_buffers,
            numa_node: Some(numa_node),
        })
    }
    
    /// Get a buffer from the pool
    pub fn get_buffer(&mut self) -> Result<ZeroCopyBuffer> {
        if let Some(buffer) = self.buffers.pop() {
            return Ok(buffer);
        }
        
        // Create a new buffer
        if let Some(numa_node) = self.numa_node {
            #[cfg(target_os = "linux")]
            {
                let numa_buffer = NumaZeroCopyBuffer::new(self.buffer_size, numa_node)?;
                Ok(numa_buffer.inner().clone())
            }
            
            #[cfg(not(target_os = "linux"))]
            {
                ZeroCopyBuffer::new(self.buffer_size)
            }
        } else {
            ZeroCopyBuffer::new(self.buffer_size)
        }
    }
    
    /// Return a buffer to the pool
    pub fn return_buffer(&mut self, mut buffer: ZeroCopyBuffer) {
        if self.buffers.len() < self.max_buffers {
            // Reset buffer
            unsafe {
                buffer.set_len(0);
            }
            
            self.buffers.push(buffer);
        }
        // If the pool is full, the buffer will be dropped
    }
    
    /// Get the number of buffers in the pool
    pub fn available_buffers(&self) -> usize {
        self.buffers.len()
    }
    
    /// Get the maximum number of buffers
    pub fn max_buffers(&self) -> usize {
        self.max_buffers
    }
    
    /// Get the buffer size
    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }
    
    /// Get the NUMA node
    pub fn numa_node(&self) -> Option<i32> {
        self.numa_node
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_copy_buffer() {
        let mut buffer = ZeroCopyBuffer::new(1024).unwrap();
        
        // Test initial state
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
        assert!(buffer.capacity() >= 1024);
        
        // Test copy_from_slice
        let data = b"Hello, world!";
        buffer.copy_from_slice(data).unwrap();
        
        assert_eq!(buffer.len(), data.len());
        assert!(!buffer.is_empty());
        assert_eq!(&buffer[..], data);
        
        // Test from_slice
        let buffer2 = ZeroCopyBuffer::from_slice(data).unwrap();
        assert_eq!(buffer2.len(), data.len());
        assert_eq!(&buffer2[..], data);
        
        // Test resize
        buffer.resize(2048).unwrap();
        assert!(buffer.capacity() >= 2048);
        assert_eq!(buffer.len(), data.len());
        assert_eq!(&buffer[..], data);
    }

    #[test]
    fn test_zero_copy_ring_buffer() {
        let mut buffer = ZeroCopyRingBuffer::new(16).unwrap();
        
        // Test initial state
        assert!(buffer.is_empty());
        assert!(!buffer.is_full());
        assert_eq!(buffer.available_read(), 0);
        assert_eq!(buffer.available_write(), 16);
        
        // Test write
        let data1 = b"Hello";
        let written = buffer.write(data1).unwrap();
        assert_eq!(written, 5);
        assert!(!buffer.is_empty());
        assert!(!buffer.is_full());
        assert_eq!(buffer.available_read(), 5);
        assert_eq!(buffer.available_write(), 11);
        
        // Test read
        let mut read_buf = [0u8; 3];
        let read = buffer.read(&mut read_buf).unwrap();
        assert_eq!(read, 3);
        assert_eq!(&read_buf, b"Hel");
        assert!(!buffer.is_empty());
        assert!(!buffer.is_full());
        assert_eq!(buffer.available_read(), 2);
        assert_eq!(buffer.available_write(), 14);
        
        // Test wrap-around
        let data2 = b"World!";
        let written = buffer.write(data2).unwrap();
        assert_eq!(written, 6);
        assert!(!buffer.is_empty());
        assert!(!buffer.is_full());
        assert_eq!(buffer.available_read(), 8);
        assert_eq!(buffer.available_write(), 8);
        
        // Read remaining data
        let mut read_buf = [0u8; 8];
        let read = buffer.read(&mut read_buf).unwrap();
        assert_eq!(read, 8);
        assert_eq!(&read_buf, b"loWorld!");
        assert!(buffer.is_empty());
        assert!(!buffer.is_full());
        assert_eq!(buffer.available_read(), 0);
        assert_eq!(buffer.available_write(), 16);
        
        // Test full buffer
        let data3 = [1u8; 16];
        let written = buffer.write(&data3).unwrap();
        assert_eq!(written, 16);
        assert!(!buffer.is_empty());
        assert!(buffer.is_full());
        assert_eq!(buffer.available_read(), 16);
        assert_eq!(buffer.available_write(), 0);
        
        // Test clear
        buffer.clear();
        assert!(buffer.is_empty());
        assert!(!buffer.is_full());
        assert_eq!(buffer.available_read(), 0);
        assert_eq!(buffer.available_write(), 16);
    }
    
    #[test]
    fn test_zero_copy_memory_pool() {
        let mut pool = ZeroCopyMemoryPool::new(1024, 5).unwrap();
        
        // Test initial state
        assert_eq!(pool.available_buffers(), 0);
        assert_eq!(pool.max_buffers(), 5);
        assert_eq!(pool.buffer_size(), 1024);
        assert_eq!(pool.numa_node(), None);
        
        // Get buffers
        let buffer1 = pool.get_buffer().unwrap();
        let buffer2 = pool.get_buffer().unwrap();
        
        assert_eq!(pool.available_buffers(), 0);
        
        // Return buffers
        pool.return_buffer(buffer1);
        pool.return_buffer(buffer2);
        
        assert_eq!(pool.available_buffers(), 2);
        
        // Get buffers again
        let _buffer1 = pool.get_buffer().unwrap();
        let _buffer2 = pool.get_buffer().unwrap();
        
        assert_eq!(pool.available_buffers(), 0);
    }
}

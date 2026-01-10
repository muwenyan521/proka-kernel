//! Page protection flags and utilities
//! Copyright (C) RainSTR Studio 2025, All Rights Reserved.
//!
//! This module provides page protection bit definitions and utilities
//! for managing memory access permissions.
//!
//! # Overview
//!
//! The protection module defines a high-level abstraction for memory page
//! protection flags, providing a type-safe interface for managing memory
//! access permissions. It bridges between the human-readable [`Protection`]
//! enumeration and the low-level x86_64 page table flags.
//!
//! # Examples
//!
//! ```
//! use kernel::memory::protection::{Protection, kernel_flags, user_flags};
//! use x86_64::structures::paging::PageTableFlags;
//!
//! // Create kernel page flags
//! let kernel_read_write = kernel_flags(true);
//! let kernel_read_only = kernel_flags(false);
//!
//! // Create user page flags with specific protection
//! let user_read_execute = user_flags(Protection::ReadExecute);
//! let user_read_write = user_flags(Protection::ReadWrite);
//!
//! // Check protection permissions
//! assert!(Protection::ReadWrite.can_read());
//! assert!(Protection::ReadWrite.can_write());
//! assert!(!Protection::ReadWrite.can_execute());
//! ```

use x86_64::structures::paging::PageTableFlags;

/// Page protection flags with human-readable descriptions
///
/// This enumeration provides a high-level abstraction for memory page
/// protection permissions, mapping to x86_64 page table flags.
///
/// # Variants
///
/// - [`ReadExecute`](Protection::ReadExecute): Read-only and executable memory
/// - [`Read`](Protection::Read): Read-only, non-executable memory
/// - [`ReadWriteExecute`](Protection::ReadWriteExecute): Read-write and executable memory
/// - [`ReadWrite`](Protection::ReadWrite): Read-write, non-executable memory
/// - [`None`](Protection::None): No access (page not present)
///
/// # Safety
///
/// The protection flags must be used correctly to maintain memory safety.
/// Incorrect protection settings can lead to security vulnerabilities or
/// undefined behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protection {
    /// Read-only, executable memory
    ///
    /// Used for code sections that should not be modified at runtime.
    ReadExecute,
    /// Read-only, non-executable memory
    ///
    /// Used for constant data that should not be modified or executed.
    Read,
    /// Read-write, executable memory
    ///
    /// Used for self-modifying code or JIT-compiled code regions.
    /// Use with caution as it poses security risks.
    ReadWriteExecute,
    /// Read-write, non-executable memory
    ///
    /// Used for normal data sections that can be read and written.
    ReadWrite,
    /// No access (page not present)
    ///
    /// Represents an unmapped page or a page that has been explicitly
    /// marked as inaccessible.
    None,
}

impl Protection {
    /// Convert protection to x86_64 page table flags
    ///
    /// # Returns
    ///
    /// Returns the corresponding [`PageTableFlags`] for this protection level.
    ///
    /// # Examples
    ///
    /// ```
    /// use kernel::memory::protection::Protection;
    /// use x86_64::structures::paging::PageTableFlags;
    ///
    /// let flags = Protection::ReadWrite.to_flags();
    /// assert!(flags.contains(PageTableFlags::PRESENT));
    /// assert!(flags.contains(PageTableFlags::WRITABLE));
    /// assert!(flags.contains(PageTableFlags::USER_ACCESSIBLE));
    /// assert!(flags.contains(PageTableFlags::NO_EXECUTE));
    /// ```
    pub fn to_flags(self) -> PageTableFlags {
        match self {
            Protection::ReadExecute => {
                PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE
            }
            Protection::Read => {
                PageTableFlags::PRESENT
                    | PageTableFlags::USER_ACCESSIBLE
                    | PageTableFlags::NO_EXECUTE
            }
            Protection::ReadWriteExecute => {
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE
            }
            Protection::ReadWrite => {
                PageTableFlags::PRESENT
                    | PageTableFlags::WRITABLE
                    | PageTableFlags::USER_ACCESSIBLE
                    | PageTableFlags::NO_EXECUTE
            }
            Protection::None => PageTableFlags::empty(),
        }
    }

    /// Check if protection allows reading
    ///
    /// # Returns
    ///
    /// Returns `true` if the protection level permits read access.
    ///
    /// # Examples
    ///
    /// ```
    /// use kernel::memory::protection::Protection;
    ///
    /// assert!(Protection::Read.can_read());
    /// assert!(Protection::ReadWrite.can_read());
    /// assert!(!Protection::None.can_read());
    /// ```
    pub fn can_read(self) -> bool {
        matches!(self, Protection::Read | Protection::ReadWrite | Protection::ReadExecute | Protection::ReadWriteExecute)
    }

    /// Check if protection allows writing
    ///
    /// # Returns
    ///
    /// Returns `true` if the protection level permits write access.
    ///
    /// # Examples
    ///
    /// ```
    /// use kernel::memory::protection::Protection;
    ///
    /// assert!(Protection::ReadWrite.can_write());
    /// assert!(!Protection::Read.can_write());
    /// assert!(!Protection::None.can_write());
    /// ```
    pub fn can_write(self) -> bool {
        matches!(self, Protection::ReadWrite | Protection::ReadWriteExecute)
    }

    /// Check if protection allows execution
    ///
    /// # Returns
    ///
    /// Returns `true` if the protection level permits execution.
    ///
    /// # Examples
    ///
    /// ```
    /// use kernel::memory::protection::Protection;
    ///
    /// assert!(Protection::ReadExecute.can_execute());
    /// assert!(!Protection::Read.can_execute());
    /// assert!(!Protection::None.can_execute());
    /// ```
    pub fn can_execute(self) -> bool {
        matches!(self, Protection::ReadExecute | Protection::ReadWriteExecute)
    }

    /// Get protection description
    ///
    /// # Returns
    ///
    /// Returns a human-readable string describing the protection level.
    ///
    /// # Examples
    ///
    /// ```
    /// use kernel::memory::protection::Protection;
    ///
    /// assert_eq!(Protection::Read.description(), "read-only");
    /// assert_eq!(Protection::ReadWriteExecute.description(), "read+write+execute");
    /// ```
    pub fn description(self) -> &'static str {
        match self {
            Protection::ReadExecute => "read+execute",
            Protection::Read => "read-only",
            Protection::ReadWriteExecute => "read+write+execute",
            Protection::ReadWrite => "read+write",
            Protection::None => "no access",
        }
    }
}

impl From<PageTableFlags> for Protection {
    /// Convert x86_64 page table flags to protection level
    ///
    /// # Arguments
    ///
    /// * `flags` - The page table flags to convert
    ///
    /// # Returns
    ///
    /// Returns the corresponding [`Protection`] level.
    ///
    /// # Examples
    ///
    /// ```
    /// use kernel::memory::protection::Protection;
    /// use x86_64::structures::paging::PageTableFlags;
    ///
    /// let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE;
    /// let protection = Protection::from(flags);
    /// assert_eq!(protection, Protection::ReadWrite);
    /// ```
    fn from(flags: PageTableFlags) -> Self {
        if !flags.contains(PageTableFlags::PRESENT) {
            return Protection::None;
        }

        let writable = flags.contains(PageTableFlags::WRITABLE);
        let no_execute = flags.contains(PageTableFlags::NO_EXECUTE);

        match (writable, no_execute) {
            (false, false) => Protection::ReadExecute,
            (false, true) => Protection::Read,
            (true, false) => Protection::ReadWriteExecute,
            (true, true) => Protection::ReadWrite,
        }
    }
}

/// Create kernel page flags (always executable, can be read-write or read-only)
///
/// Kernel pages are always executable and never have the `NO_EXECUTE` flag set.
/// They also never have the `USER_ACCESSIBLE` flag set.
///
/// # Arguments
///
/// * `writable` - If `true`, the page will be writable; otherwise read-only
///
/// # Returns
///
/// Returns the appropriate [`PageTableFlags`] for kernel memory.
///
/// # Examples
///
/// ```
/// use kernel::memory::protection::kernel_flags;
/// use x86_64::structures::paging::PageTableFlags;
///
/// let read_only = kernel_flags(false);
/// assert!(read_only.contains(PageTableFlags::PRESENT));
/// assert!(!read_only.contains(PageTableFlags::WRITABLE));
///
/// let read_write = kernel_flags(true);
/// assert!(read_write.contains(PageTableFlags::PRESENT));
/// assert!(read_write.contains(PageTableFlags::WRITABLE));
/// ```
pub fn kernel_flags(writable: bool) -> PageTableFlags {
    let mut flags = PageTableFlags::PRESENT;
    if writable {
        flags |= PageTableFlags::WRITABLE;
    }
    flags
}

/// Create user page flags with specified protection
///
/// User pages always have the `USER_ACCESSIBLE` flag set and respect
/// the execution prevention flag based on the protection level.
///
/// # Arguments
///
/// * `protection` - The desired protection level for user memory
///
/// # Returns
///
/// Returns the appropriate [`PageTableFlags`] for user memory.
///
/// # Examples
///
/// ```
/// use kernel::memory::protection::{Protection, user_flags};
/// use x86_64::structures::paging::PageTableFlags;
///
/// let flags = user_flags(Protection::Read);
/// assert!(flags.contains(PageTableFlags::PRESENT));
/// assert!(flags.contains(PageTableFlags::USER_ACCESSIBLE));
/// assert!(flags.contains(PageTableFlags::NO_EXECUTE));
/// ```
pub fn user_flags(protection: Protection) -> PageTableFlags {
    protection.to_flags()
}

/// Create read-only page flags (for both kernel and user)
///
/// These flags create a read-only, non-executable page. For user pages,
/// you should use [`user_flags`] with [`Protection::Read`] instead.
///
/// # Returns
///
/// Returns [`PageTableFlags`] for a read-only, non-executable page.
///
/// # Examples
///
/// ```
/// use kernel::memory::protection::read_only_flags;
/// use x86_64::structures::paging::PageTableFlags;
///
/// let flags = read_only_flags();
/// assert!(flags.contains(PageTableFlags::PRESENT));
/// assert!(flags.contains(PageTableFlags::NO_EXECUTE));
/// assert!(!flags.contains(PageTableFlags::WRITABLE));
/// ```
pub fn read_only_flags() -> PageTableFlags {
    PageTableFlags::PRESENT | PageTableFlags::NO_EXECUTE
}

/// Create writable page flags (for both kernel and user)
///
/// These flags create a read-write, non-executable page. For user pages,
/// you should use [`user_flags`] with [`Protection::ReadWrite`] instead.
///
/// # Returns
///
/// Returns [`PageTableFlags`] for a read-write, non-executable page.
///
/// # Examples
///
/// ```
/// use kernel::memory::protection::writable_flags;
/// use x86_64::structures::paging::PageTableFlags;
///
/// let flags = writable_flags();
/// assert!(flags.contains(PageTableFlags::PRESENT));
/// assert!(flags.contains(PageTableFlags::WRITABLE));
/// assert!(flags.contains(PageTableFlags::NO_EXECUTE));
/// ```
pub fn writable_flags() -> PageTableFlags {
    PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE
}

/// Create executable page flags
///
/// These flags create an executable page without write permission.
/// For user pages, you should use [`user_flags`] with [`Protection::ReadExecute`] instead.
///
/// # Returns
///
/// Returns [`PageTableFlags`] for an executable, read-only page.
///
/// # Examples
///
/// ```
/// use kernel::memory::protection::executable_flags;
/// use x86_64::structures::paging::PageTableFlags;
///
/// let flags = executable_flags();
/// assert!(flags.contains(PageTableFlags::PRESENT));
/// assert!(!flags.contains(PageTableFlags::WRITABLE));
/// assert!(!flags.contains(PageTableFlags::NO_EXECUTE));
/// ```
pub fn executable_flags() -> PageTableFlags {
    PageTableFlags::PRESENT
}

/// Create read-write-executable page flags
///
/// These flags create a read-write, executable page. For user pages,
/// you should use [`user_flags`] with [`Protection::ReadWriteExecute`] instead.
///
/// # Returns
///
/// Returns [`PageTableFlags`] for a read-write, executable page.
///
/// # Examples
///
/// ```
/// use kernel::memory::protection::read_write_execute_flags;
/// use x86_64::structures::paging::PageTableFlags;
///
/// let flags = read_write_execute_flags();
/// assert!(flags.contains(PageTableFlags::PRESENT));
/// assert!(flags.contains(PageTableFlags::WRITABLE));
/// assert!(!flags.contains(PageTableFlags::NO_EXECUTE));
/// ```
pub fn read_write_execute_flags() -> PageTableFlags {
    PageTableFlags::PRESENT | PageTableFlags::WRITABLE
}

/// Check if page table flags allow read access
///
/// # Arguments
///
/// * `flags` - The page table flags to check
///
/// # Returns
///
/// Returns `true` if the flags permit read access.
///
/// # Examples
///
/// ```
/// use kernel::memory::protection::flags_can_read;
/// use x86_64::structures::paging::PageTableFlags;
///
/// let present_flags = PageTableFlags::PRESENT;
/// assert!(flags_can_read(present_flags));
///
/// let empty_flags = PageTableFlags::empty();
/// assert!(!flags_can_read(empty_flags));
/// ```
pub fn flags_can_read(flags: PageTableFlags) -> bool {
    flags.contains(PageTableFlags::PRESENT)
}

/// Check if page table flags allow write access
///
/// # Arguments
///
/// * `flags` - The page table flags to check
///
/// # Returns
///
/// Returns `true` if the flags permit write access.
///
/// # Examples
///
/// ```
/// use kernel::memory::protection::flags_can_write;
/// use x86_64::structures::paging::PageTableFlags;
///
/// let writable_flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
/// assert!(flags_can_write(writable_flags));
///
/// let read_only_flags = PageTableFlags::PRESENT;
/// assert!(!flags_can_write(read_only_flags));
/// ```
pub fn flags_can_write(flags: PageTableFlags) -> bool {
    flags.contains(PageTableFlags::PRESENT) && flags.contains(PageTableFlags::WRITABLE)
}

/// Check if page table flags allow execute access
///
/// # Arguments
///
/// * `flags` - The page table flags to check
///
/// # Returns
///
/// Returns `true` if the flags permit execute access.
///
/// # Examples
///
/// ```
/// use kernel::memory::protection::flags_can_execute;
/// use x86_64::structures::paging::PageTableFlags;
///
/// let executable_flags = PageTableFlags::PRESENT;
/// assert!(flags_can_execute(executable_flags));
///
/// let no_execute_flags = PageTableFlags::PRESENT | PageTableFlags::NO_EXECUTE;
/// assert!(!flags_can_execute(no_execute_flags));
/// ```
pub fn flags_can_execute(flags: PageTableFlags) -> bool {
    flags.contains(PageTableFlags::PRESENT) && !flags.contains(PageTableFlags::NO_EXECUTE)
}

//! Page protection flags and utilities
//! Copyright (C) RainSTR Studio 2025, All Rights Reserved.
//!
//! This module provides page protection bit definitions and utilities
//! for managing memory access permissions.

use x86_64::structures::paging::PageTableFlags;

/// Page protection flags with human-readable descriptions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protection {
    /// Read-only, executable
    ReadExecute,
    /// Read-only, non-executable
    Read,
    /// Read-write, executable
    ReadWriteExecute,
    /// Read-write, non-executable
    ReadWrite,
    /// No access (page not present)
    None,
}

impl Protection {
    /// Convert protection to x86_64 page table flags
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
    pub fn can_read(self) -> bool {
        matches!(self, Protection::Read | Protection::ReadWrite | Protection::ReadExecute | Protection::ReadWriteExecute)
    }

    /// Check if protection allows writing
    pub fn can_write(self) -> bool {
        matches!(self, Protection::ReadWrite | Protection::ReadWriteExecute)
    }

    /// Check if protection allows execution
    pub fn can_execute(self) -> bool {
        matches!(self, Protection::ReadExecute | Protection::ReadWriteExecute)
    }

    /// Get protection description
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
pub fn kernel_flags(writable: bool) -> PageTableFlags {
    let mut flags = PageTableFlags::PRESENT;
    if writable {
        flags |= PageTableFlags::WRITABLE;
    }
    flags
}

/// Create user page flags with specified protection
pub fn user_flags(protection: Protection) -> PageTableFlags {
    protection.to_flags()
}

/// Create read-only page flags (for both kernel and user)
pub fn read_only_flags() -> PageTableFlags {
    PageTableFlags::PRESENT | PageTableFlags::NO_EXECUTE
}

/// Create writable page flags (for both kernel and user)
pub fn writable_flags() -> PageTableFlags {
    PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE
}

/// Create executable page flags
pub fn executable_flags() -> PageTableFlags {
    PageTableFlags::PRESENT
}

/// Create read-write-executable page flags
pub fn read_write_execute_flags() -> PageTableFlags {
    PageTableFlags::PRESENT | PageTableFlags::WRITABLE
}

/// Check if page table flags allow read access
pub fn flags_can_read(flags: PageTableFlags) -> bool {
    flags.contains(PageTableFlags::PRESENT)
}

/// Check if page table flags allow write access
pub fn flags_can_write(flags: PageTableFlags) -> bool {
    flags.contains(PageTableFlags::PRESENT) && flags.contains(PageTableFlags::WRITABLE)
}

/// Check if page table flags allow execute access
pub fn flags_can_execute(flags: PageTableFlags) -> bool {
    flags.contains(PageTableFlags::PRESENT) && !flags.contains(PageTableFlags::NO_EXECUTE)
}

// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Pistonight/pvz-bintools contributors

use std::fs::{File, OpenOptions};
use std::os::windows::io::AsRawHandle;
use std::path::Path;

use chrono::DateTime;
use cu::pre::*;
use windows_sys::Win32::Foundation::{FILETIME, GetLastError};
use windows_sys::Win32::Storage::FileSystem::{GetFileTime, SetFileTime};

#[derive(Debug, Default, Clone, Copy)]
pub struct WinFileTime {
    pub dw_lo: u32,
    pub dw_hi: u32,
}

impl From<FILETIME> for WinFileTime {
    fn from(value: FILETIME) -> Self {
        Self {
            dw_lo: value.dwLowDateTime,
            dw_hi: value.dwHighDateTime,
        }
    }
}

impl From<WinFileTime> for FILETIME {
    fn from(value: WinFileTime) -> Self {
        FILETIME {
            dwLowDateTime: value.dw_lo,
            dwHighDateTime: value.dw_hi,
        }
    }
}

impl std::fmt::Display for WinFileTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let intervals = (self.dw_hi as u64) << 32 | self.dw_lo as u64;
        const INTERVALS_PER_SEC: u64 = 10_000_000; // 100-ns units
        const EPOCH_DIFF_SECS: u64 = 134_774 * 86_400; // days from 1601-01-01 to 1970-01-01 × secs/day
        let unix_intervals = intervals.saturating_sub(EPOCH_DIFF_SECS * INTERVALS_PER_SEC);
        let secs = (unix_intervals / INTERVALS_PER_SEC) as i64;
        let nanos = ((unix_intervals % INTERVALS_PER_SEC) * 100) as u32;
        match DateTime::from_timestamp(secs, nanos) {
            Some(dt) => write!(f, "{}", dt.format("%Y-%m-%d %H:%M:%S UTC")),
            None => write!(f, "<invalid filetime>"),
        }
    }
}

pub fn set(path: &Path, filetime: WinFileTime) -> cu::Result<()> {
    let file = cu::check!(
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(false)
            .open(path),
        "failed to open file for setting FILETIME: '{}'",
        path.display()
    )?;
    let ft = FILETIME::from(filetime);
    let handle = file.as_raw_handle();
    let ft_ptr = &ft as *const FILETIME;
    let ok = unsafe { SetFileTime(handle, ft_ptr, ft_ptr, ft_ptr) };
    if ok == 0 {
        let error = unsafe { GetLastError() };
        cu::bail!("SetFileTime failed: 0x{error:08x}");
    }
    Ok(())
}

pub fn get(file: &File) -> cu::Result<WinFileTime> {
    let handle = file.as_raw_handle();
    let mut ft = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let ft_ptr = &mut ft as *mut FILETIME;
    let ok = unsafe { GetFileTime(handle, ft_ptr, ft_ptr, ft_ptr) };
    if ok == 0 {
        let error = unsafe { GetLastError() };
        cu::bail!("GetFileTime failed: 0x{error:08x}");
    }
    Ok(ft.into())
}

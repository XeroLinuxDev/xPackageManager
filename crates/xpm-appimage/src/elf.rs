//! Minimal ELF64 reader used to detect AppImage update capability.
//!
//! Type-2 AppImages are ELF executables that embed update information in a
//! section named `.upd_info` (format e.g. `zsync|URL` or `gh-releases-zsync|...`).
//! A present, non-empty section means AppImageUpdate/zsync can update the file.
//! We only parse little-endian ELF64 (x86_64/aarch64); anything else returns None.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

fn read_u16(f: &mut File, off: u64) -> Option<u16> {
    let mut b = [0u8; 2];
    f.seek(SeekFrom::Start(off)).ok()?;
    f.read_exact(&mut b).ok()?;
    Some(u16::from_le_bytes(b))
}

fn read_u32(f: &mut File, off: u64) -> Option<u32> {
    let mut b = [0u8; 4];
    f.seek(SeekFrom::Start(off)).ok()?;
    f.read_exact(&mut b).ok()?;
    Some(u32::from_le_bytes(b))
}

fn read_u64(f: &mut File, off: u64) -> Option<u64> {
    let mut b = [0u8; 8];
    f.seek(SeekFrom::Start(off)).ok()?;
    f.read_exact(&mut b).ok()?;
    Some(u64::from_le_bytes(b))
}

/// Return the contents of the `.upd_info` ELF section, if present and non-empty.
pub fn read_upd_info(path: &Path) -> Option<String> {
    let mut f = File::open(path).ok()?;

    let mut ident = [0u8; 16];
    f.read_exact(&mut ident).ok()?;
    // Magic 0x7f 'E' 'L' 'F', class 2 (64-bit), data 1 (little-endian).
    if &ident[0..4] != b"\x7fELF" || ident[4] != 2 || ident[5] != 1 {
        return None;
    }

    // ELF64 header offsets.
    let e_shoff = read_u64(&mut f, 0x28)?;
    let e_shentsize = read_u16(&mut f, 0x3a)? as u64;
    let e_shnum = read_u16(&mut f, 0x3c)? as u64;
    let e_shstrndx = read_u16(&mut f, 0x3e)? as u64;
    if e_shoff == 0 || e_shnum == 0 || e_shentsize < 64 || e_shstrndx >= e_shnum {
        return None;
    }

    // Section header string table: locate via the shstrndx section header.
    let shstr_hdr = e_shoff + e_shstrndx * e_shentsize;
    let shstr_off = read_u64(&mut f, shstr_hdr + 0x18)?;
    let shstr_size = read_u64(&mut f, shstr_hdr + 0x20)?;
    if shstr_size == 0 || shstr_size > 1_048_576 {
        return None;
    }
    let mut strtab = vec![0u8; shstr_size as usize];
    f.seek(SeekFrom::Start(shstr_off)).ok()?;
    f.read_exact(&mut strtab).ok()?;

    for i in 0..e_shnum {
        let hdr = e_shoff + i * e_shentsize;
        let sh_name = read_u32(&mut f, hdr)? as usize;
        // Read the NUL-terminated name out of the string table.
        let end = strtab[sh_name..].iter().position(|&b| b == 0)
            .map(|p| sh_name + p)
            .unwrap_or(strtab.len());
        if &strtab[sh_name..end] != b".upd_info" {
            continue;
        }

        let sh_offset = read_u64(&mut f, hdr + 0x18)?;
        let sh_size = read_u64(&mut f, hdr + 0x20)?;
        if sh_size == 0 || sh_size > 8192 {
            return None;
        }
        let mut buf = vec![0u8; sh_size as usize];
        f.seek(SeekFrom::Start(sh_offset)).ok()?;
        f.read_exact(&mut buf).ok()?;

        // Trim trailing NUL padding; reject all-zero (placeholder) sections.
        let trimmed: Vec<u8> = buf.into_iter().take_while(|&b| b != 0).collect();
        if trimmed.is_empty() {
            return None;
        }
        return String::from_utf8(trimmed).ok().filter(|s| !s.trim().is_empty());
    }

    None
}

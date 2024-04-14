#![no_std]
//! A crate to parse a Flattened Device Tree (FDT)
//! into a structure intended for immediate consupmtion
//! by the operating system.
use core::ffi::CStr;
use core::num;
use core::slice;
use core::mem;

const FDT_BEGIN_NODE: u32 = 0x00000001_u32;
const FDT_END_NODE: u32 =  0x00000002_u32;
const FDT_PROP: u32 = 0x00000003_u32;
const FDT_NOP: u32 = 0x00000004_u32;
const FDT_END: u32 = 0x00000009_u32;

const FDT_HDR_MAGIC: u32 = 0xd00dfeed_u32;

#[repr(C)]
#[derive(Debug)]
pub struct FdtHeader {
    magic: u32,
    totalsize: u32,
    off_dt_struct: u32,
    off_dt_strings: u32,
    off_mem_rsvmap: u32,
    version: u32,
    last_comp_version: u32,
    boot_cpuid_phys: u32,
    size_dt_strings: u32,
    size_dt_struct: u32
}

#[derive(Debug)]
pub struct Fdt<'a> {
    pub header: FdtHeader,
    reserved_memory: &'a[FdtReserveEntry],
    pub dt_struct: &'a [u8],
    pub dt_strings: &'a [u8]
}

#[repr(C)]
#[derive(Debug)]
pub struct FdtReserveEntry {
    address: u64,
    size: u64
}

#[repr(C)]
pub struct FdtProp {
    len: u32,
    nameoff: u32
}

#[derive(Debug)]
pub enum FdtError {
    InvalidMagic,
    InvalidPointer,
    NotFound,
}

impl Fdt<'_> {
    pub fn new(fdt_addr: *const u8) -> Result<Self, FdtError> {
        if fdt_addr == 0 as *const u8 { return Err(FdtError::InvalidPointer) }
        let hdr = Fdt::_parse_header(fdt_addr)?;
        let (mem_reserve, hdr) = Fdt::_parse_mem_reserve(fdt_addr,hdr);
        let (dt_struct, hdr) = Fdt::_parse_dt_struct(fdt_addr, hdr);
        let (dt_strings, hdr) = Fdt::_parse_dt_strings(fdt_addr, hdr);
        let fdt = Fdt { 
            header: hdr,
            reserved_memory: mem_reserve,
            dt_struct: dt_struct,
            dt_strings: dt_strings
        };
        return Ok(fdt);
    }

    pub fn get_reserved_memory_regions(&self) -> impl Iterator<Item = (u64, u64)> + '_ {
        self.reserved_memory.iter().map(|x| (x.address.to_be(), x.size.to_be()))
    }

    pub fn st(&self) -> impl Iterator<Item = &u8> + '_ {
        self.dt_strings.iter()
    }

    pub fn get_string(&self, offset: usize) -> Option<&str> {
        if offset < self.header.size_dt_strings as usize {
            let sl = &self.dt_strings[offset..self.header.size_dt_strings as usize];
            let cstr = CStr::from_bytes_until_nul(sl).unwrap();
            let str = cstr.to_str().unwrap();
            Some(str)
        } else {
            None
        }
    }

    fn _parse_dt_struct(fdt_addr: *const u8, fdt_hdr: FdtHeader) -> (&'static [u8], FdtHeader) {
        unsafe {
            let dt_struct = slice::from_raw_parts(fdt_addr.add(fdt_hdr.off_dt_struct as usize),
                                                        fdt_hdr.size_dt_struct as usize);
            (dt_struct,fdt_hdr)
        }
    }

    fn _parse_dt_strings(fdt_addr: *const u8, fdt_hdr: FdtHeader) -> (&'static [u8], FdtHeader) {
        unsafe {
            let dt_strings = slice::from_raw_parts(fdt_addr.add(fdt_hdr.off_dt_strings as usize),
            fdt_hdr.size_dt_strings as usize);
            (dt_strings, fdt_hdr)
        }
    }

    fn _parse_mem_reserve(fdt_addr: *const u8, fdt_hdr: FdtHeader) -> (&'static [FdtReserveEntry], FdtHeader) {
        let mut mem_reserve;
        unsafe {
            mem_reserve = slice::from_raw_parts(fdt_addr
                                                    .add(fdt_hdr.off_mem_rsvmap as usize) as *const FdtReserveEntry,
                                                    ((fdt_hdr.off_dt_struct-fdt_hdr.off_mem_rsvmap)
                                                    /mem::size_of::<FdtReserveEntry>() as u32) as usize);
        }
        let mem_iter = mem_reserve.iter();
        for (len, entry) in  mem_iter.enumerate() {
            if entry.address == 0 && entry.size == 0 { 
                unsafe {
                    mem_reserve = slice::from_raw_parts(fdt_addr
                        .add(fdt_hdr.off_mem_rsvmap as usize) as *const FdtReserveEntry, len);
                }
                break;
            }
        }
        (mem_reserve,fdt_hdr)
    }

    fn _parse_header(fdt_addr: *const u8) -> Result<FdtHeader, FdtError> {
        let fdt_hdr;
        unsafe {
            fdt_hdr = slice::from_raw_parts(fdt_addr as *const FdtHeader, 1);
        }
        if let Some(fdt_hdr) = fdt_hdr.first() {
            match fdt_hdr.magic.to_be() {
                FDT_HDR_MAGIC => {
                    let hdr = FdtHeader {
                        magic: fdt_hdr.magic.to_be(),
                        totalsize: fdt_hdr.totalsize.to_be(),
                        off_dt_struct: fdt_hdr.off_dt_struct.to_be(),
                        off_dt_strings: fdt_hdr.off_dt_strings.to_be(),
                        off_mem_rsvmap: fdt_hdr.off_mem_rsvmap.to_be(),
                        version: fdt_hdr.version.to_be(),
                        last_comp_version: fdt_hdr.last_comp_version.to_be(),
                        boot_cpuid_phys: fdt_hdr.boot_cpuid_phys.to_be(),
                        size_dt_strings: fdt_hdr.size_dt_strings.to_be(),
                        size_dt_struct: fdt_hdr.size_dt_struct.to_be()
                    };
                    return Ok(hdr);
                }
                _ => { return Err(FdtError::InvalidMagic); }
            }
        }
        Err(FdtError::NotFound)
    }
}

/*#[cfg(test)]
mod tests {
    use std::{fs::{self, File}, io::Read};

    use super::*;

    #[test]
    fn it_works() {
        let filename = "/home/arryn/fdt/t8103-j313.dtb";
        let mut f = File::open(filename).unwrap();
        let metadata = fs::metadata(&filename).expect("unable to read metadata");
        let mut buffer = vec![0; metadata.len() as usize];
        f.read(&mut buffer).expect("buffer overflow");
        let memory_regions = [
            FdtReserveEntry {address: 0x100_u64.to_be(), size: 0x600_u64.to_be()},
            FdtReserveEntry {address: 0x200_u64.to_be(), size: 0x700_u64.to_be()},
            FdtReserveEntry {address: 0x300_u64.to_be(), size: 0x800_u64.to_be()},
            FdtReserveEntry {address: 0x400_u64.to_be(), size: 0x900_u64.to_be()},
            FdtReserveEntry {address: 0x500_u64.to_be(), size: 0xA00_u64.to_be()},
            FdtReserveEntry {address: 0x000_u64.to_be(), size: 0x000_u64.to_be()},
            ];

        let mut fdt = Fdt::new(buffer.as_ptr()).unwrap();
        fdt.reserved_memory = &memory_regions;
        for x in fdt.get_reserved_memory_regions() {
            let m = x;
            println!("{m:#x?}");
        }
    }
}*/

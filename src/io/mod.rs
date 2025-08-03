use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use memmap2::{Mmap, MmapOptions};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::converter::{ISO_SECTOR_SIZE, RAW_SECTOR_SIZE};

pub struct IsoReader {
    mmap: Mmap,
    total_sectors: usize,
}

impl IsoReader {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(&path)
            .with_context(|| format!("Failed to open ISO file: {}", path.as_ref().display()))?;

        let metadata = file.metadata()?;
        let file_size = metadata.len() as usize;

        if !file_size.is_multiple_of(ISO_SECTOR_SIZE) {
            anyhow::bail!(
                "Invalid ISO file size: {} is not a multiple of {}",
                file_size,
                ISO_SECTOR_SIZE
            );
        }

        let mmap = unsafe {
            MmapOptions::new()
                .map(&file)
                .with_context(|| "Failed to memory-map ISO file")?
        };

        Ok(Self {
            mmap,
            total_sectors: file_size / ISO_SECTOR_SIZE,
        })
    }

    pub fn total_sectors(&self) -> usize {
        self.total_sectors
    }

    pub fn read_sector(&self, sector_index: usize) -> Option<&[u8]> {
        if sector_index >= self.total_sectors {
            return None;
        }

        let offset = sector_index * ISO_SECTOR_SIZE;
        Some(&self.mmap[offset..offset + ISO_SECTOR_SIZE])
    }
}

pub struct RawWriter {
    writer: BufWriter<File>,
    sectors_written: usize,
}

impl RawWriter {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .with_context(|| {
                format!("Failed to create output file: {}", path.as_ref().display())
            })?;

        Ok(Self {
            writer: BufWriter::with_capacity(1024 * 1024, file), // 1MB buffer
            sectors_written: 0,
        })
    }

    pub fn write_sector(&mut self, data: &[u8]) -> Result<()> {
        if data.len() != RAW_SECTOR_SIZE {
            anyhow::bail!(
                "Invalid RAW sector size: expected {}, got {}",
                RAW_SECTOR_SIZE,
                data.len()
            );
        }

        self.writer.write_all(data)?;
        self.sectors_written += 1;
        Ok(())
    }
}

pub fn create_progress_bar(total_sectors: usize) -> ProgressBar {
    let pb = ProgressBar::new(total_sectors as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} sectors ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb
}

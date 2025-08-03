pub mod converter;
pub mod edc_ecc;
mod io;
mod parallel;

use std::path::PathBuf;
use std::time::Instant;
use anyhow::Result;
use clap::Parser;
use rayon::prelude::*;

use crate::io::{IsoReader, RawWriter, create_progress_bar};
use crate::parallel::ParallelProcessor;
use crate::converter::{convert_iso_to_raw, ISO_SECTOR_SIZE, RAW_SECTOR_SIZE};

#[derive(Parser, Debug)]
#[command(name = "iso2raw")]
#[command(about = "Convert ISO files to RAW (MODE1/2352) format", long_about = None)]
struct Args {
    /// Input ISO file path
    #[arg(value_name = "INPUT")]
    input: PathBuf,
    
    /// Output RAW file path (defaults to input with .bin extension)
    #[arg(short, long, value_name = "OUTPUT")]
    output: Option<PathBuf>,
    
    /// Number of worker threads (defaults to number of CPU cores)
    #[arg(short = 'j', long)]
    threads: Option<usize>,
    
    /// Disable progress bar
    #[arg(short, long)]
    quiet: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    // Determine output path
    let output_path = args.output.unwrap_or_else(|| {
        let mut path = args.input.clone();
        path.set_extension("bin");
        path
    });
    
    // Validate input
    if !args.input.exists() {
        anyhow::bail!("Input file does not exist: {}", args.input.display());
    }
    
    if args.input == output_path {
        anyhow::bail!("Input and output files cannot be the same");
    }
    
    println!("Converting {} to {}", args.input.display(), output_path.display());
    
    let start_time = Instant::now();
    
    // Open input ISO
    let iso_reader = IsoReader::new(&args.input)?;
    let total_sectors = iso_reader.total_sectors();
    
    println!("Total sectors: {} ({:.2} MB)", 
        total_sectors, 
        (total_sectors * ISO_SECTOR_SIZE) as f64 / (1024.0 * 1024.0)
    );
    
    // Create output writer
    let mut raw_writer = RawWriter::new(&output_path)?;
    
    // Setup progress bar
    let progress = if !args.quiet {
        Some(create_progress_bar(total_sectors))
    } else {
        None
    };
    
    // Setup parallel processor
    let processor = ParallelProcessor::new(args.threads);
    println!("Using {} worker threads", processor.num_workers());
    
    // Process sectors in parallel chunks
    let chunk_size = processor.chunk_size();
    let sectors_per_batch = chunk_size * processor.num_workers();
    
    for batch_start in (0..total_sectors).step_by(sectors_per_batch) {
        let batch_end = (batch_start + sectors_per_batch).min(total_sectors);
        
        // Collect batch of sectors
        let batch: Vec<(usize, Vec<u8>)> = (batch_start..batch_end)
            .filter_map(|lba| {
                iso_reader.read_sector(lba)
                    .map(|data| (lba, data.to_vec()))
            })
            .collect();
        
        // Process batch in parallel and collect results
        let mut results: Vec<(usize, Vec<u8>)> = batch
            .into_par_iter()
            .map(|(lba, data)| {
                let raw_data = convert_iso_to_raw(lba as u32, &data).unwrap();
                (lba, raw_data)
            })
            .collect();
        
        // Sort results to maintain order
        results.sort_by_key(|(lba, _)| *lba);
        
        // Write results in order
        for (_lba, raw_data) in results {
            raw_writer.write_sector(&raw_data)?;
            
            if let Some(ref pb) = progress {
                pb.inc(1);
            }
        }
    }
    
    // Finalize progress
    if let Some(ref pb) = progress {
        pb.finish_with_message("Conversion complete");
    }
    
    let elapsed = start_time.elapsed();
    let mb_per_sec = (total_sectors * RAW_SECTOR_SIZE) as f64 / (1024.0 * 1024.0) / elapsed.as_secs_f64();
    
    println!("\nConversion completed in {:.2?} ({:.2} MB/s)", elapsed, mb_per_sec);
    println!("Output file: {}", output_path.display());
    
    Ok(())
}

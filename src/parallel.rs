pub struct ParallelProcessor {
    num_workers: usize,
    chunk_size: usize,
}

impl ParallelProcessor {
    pub fn new(num_workers: Option<usize>) -> Self {
        let num_workers = num_workers.unwrap_or_else(|| {
            let cpus = num_cpus::get();
            cpus.min(8) // Cap at 8 workers for (probably) diminishing returns
        });

        // Chunk size optimized for cache efficiency
        let chunk_size = 64; // Process 64 sectors at a time

        Self {
            num_workers,
            chunk_size,
        }
    }

    pub fn num_workers(&self) -> usize {
        self.num_workers
    }

    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }
}

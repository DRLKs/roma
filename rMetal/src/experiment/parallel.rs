use std::ops::Range;
use crate::algorithms::run_algorithms_async;

/// Internal parallel execution configuration for experiment workloads.
///
/// - `requested_threads`: explicit worker count, or `None` for auto-detection.
/// - `min_chunk_size`: lower bound used to avoid oversplitting tiny workloads.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ParallelConfig {
    requested_threads: Option<usize>,
    min_chunk_size: usize,
}

impl ParallelConfig {
    /// Creates a new config using the provided worker preference.
    pub(crate) fn new(requested_threads: Option<usize>) -> Self {
        Self {
            requested_threads,
            min_chunk_size: 1,
        }
    }

    /// Sets minimum chunk size used by the range partitioning logic.
    pub(crate) fn with_min_chunk_size(mut self, min_chunk_size: usize) -> Self {
        self.min_chunk_size = min_chunk_size.max(1);
        self
    }
}

/// Resolves effective parallelism based on workload size and configuration.
///
/// The result is always at least `1`.
fn resolve_parallelism(total_work_items: usize, config: ParallelConfig) -> usize {
    if total_work_items == 0 {
        return 1;
    }

    let requested = match config.requested_threads {
        Some(threads) => threads.max(1),
        None => std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1),
    };

    let max_by_work = total_work_items;
    let max_by_chunk = total_work_items
        .div_ceil(config.min_chunk_size.max(1))
        .max(1);

    requested.min(max_by_work).min(max_by_chunk).max(1)
}

/// Splits `[0, total)` into `workers` contiguous ranges.
fn split_ranges(total: usize, workers: usize) -> Vec<Range<usize>> {
    let mut ranges = Vec::with_capacity(workers);
    for worker_id in 0..workers {
        let start = worker_id * total / workers;
        let end = (worker_id + 1) * total / workers;
        ranges.push(start..end);
    }
    ranges
}

/// Executes one worker task per chunk and collects all results.
///
/// If `total_work_items` is small, this function may run sequentially.
/// Worker panics are propagated as panics in the caller thread.
pub(crate) fn parallel_collect_by_range<R, F>(
    total_work_items: usize,
    config: ParallelConfig,
    worker: F,
) -> Vec<R>
where
    R: Send,
    F: Fn(usize, Range<usize>) -> R + Sync,
{
    if total_work_items == 0 {
        return Vec::new();
    }

    let worker_count = resolve_parallelism(total_work_items, config);
    if worker_count <= 1 {
        return vec![worker(0, 0..total_work_items)];
    }

    let ranges = split_ranges(total_work_items, worker_count);
    let jobs: Vec<_> = ranges
        .into_iter()
        .enumerate()
        .map(|(worker_id, range)| {
            let worker_ref = &worker;
            move || worker_ref(worker_id, range)
        })
        .collect();

    run_algorithms_async(jobs)
}

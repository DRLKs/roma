use std::ops::Range;
use std::thread;

/// Resolves effective worker count for a workload.
///
/// - `requested_threads = None` uses hardware parallelism.
/// - Result is always in `[1, total_work_items]` (or `1` when total is `0`).
///
/// # Arguments
/// - `total_work_items`: total number of work units to split. If this is `0`,
///   no workers are spawned and the function returns `1` as a neutral value.
/// - `requested_threads`: worker count requested by the caller. With `Some(n)`,
///   it uses `n` (normalized to at least `1`). With `None`, it auto-detects
///   available system parallelism (`available_parallelism`).
/// - `min_chunk_size`: desired minimum chunk size. This avoids over-splitting
///   very small workloads and caps the maximum worker count to
///   `ceil(total_work_items / min_chunk_size)`.
pub(crate) fn resolve_parallelism(
    total_work_items: usize,
    requested_threads: Option<usize>,
    min_chunk_size: usize,
) -> usize {
    if total_work_items == 0 {
        return 1;
    }

    let requested = match requested_threads {
        Some(v) => v.max(1),
        None => thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1),
    };

    let max_by_work = total_work_items;
    let max_by_chunk = total_work_items.div_ceil(min_chunk_size.max(1)).max(1);

    requested.min(max_by_work).min(max_by_chunk).max(1)
}

/// Splits `[0, total)` into `workers` contiguous ranges.
pub(crate) fn split_ranges(total: usize, workers: usize) -> Vec<Range<usize>> {
    let mut ranges = Vec::with_capacity(workers);
    for worker_id in 0..workers {
        let start = worker_id * total / workers;
        let end = (worker_id + 1) * total / workers;
        ranges.push(start..end);
    }
    ranges
}

/// Maps one immutable slice in parallel preserving original element order.
///
/// This helper avoids shared mutability by generating worker-local vectors and
/// then concatenating them by chunk order.
pub(crate) fn parallel_map_indexed<T, R, F>(
    input: &[T],
    requested_threads: Option<usize>,
    min_chunk_size: usize,
    mapper: F,
) -> Vec<R>
where
    T: Sync,
    R: Send,
    F: Fn(usize, &T) -> R + Sync,
{
    let total = input.len();
    if total == 0 {
        return Vec::new();
    }

    let worker_count = resolve_parallelism(total, requested_threads, min_chunk_size);
    if worker_count <= 1 {
        return input
            .iter()
            .enumerate()
            .map(|(idx, item)| mapper(idx, item))
            .collect();
    }

    let ranges = split_ranges(total, worker_count);
    let mut ordered_chunks: Vec<Vec<R>> = Vec::with_capacity(worker_count);

    thread::scope(|scope| {
        let mut handles = Vec::with_capacity(worker_count);

        for range in ranges {
            let mapper_ref = &mapper;
            let input_ref = input;
            handles.push(scope.spawn(move || {
                let mut local = Vec::with_capacity(range.len());
                for idx in range {
                    local.push(mapper_ref(idx, &input_ref[idx]));
                }
                local
            }));
        }

        for handle in handles {
            ordered_chunks.push(
                handle
                    .join()
                    .expect("parallel worker panicked while mapping elements"),
            );
        }
    });

    let mut result = Vec::with_capacity(total);
    for mut chunk in ordered_chunks {
        result.append(&mut chunk);
    }
    result
}

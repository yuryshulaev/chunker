# chunker

Minimalistic alternative to [rayon](https://github.com/rayon-rs/rayon)’s `par_iter`/`par_chunks` + inner iteration + `reduce` for parallel processing of slices with progress bar by default.
Despite its name, this crate is really tiny: only 80 SLOC.

## Usage

```
cargo add chunker
```

Call `chunker::run` or `chunker::run_mut` with these arguments:

- `items` — slice of values to process in parallel
- `config` — configuration:`thread_count`, `chunk_size`, `progress_bar`, `bar_step`
- `init` — function to initialize a worker’s intermediate result
- `work` — `Fn` accepting an individual value from `items` and mutating the worker’s intermediate result (and/or mutating the value itself in the case of `run_mut`)
- `gather` — `FnMut` accepting an [`mpsc::Receiver`](https://doc.rust-lang.org/std/sync/mpsc/struct.Receiver.html) of workers’ intermediate results *in random order* and returns the final result (usually via `reduce`)

## Examples

Sum of squares:

```rust
chunker::run(
    &input,
    chunker::Config::default(),
    || 0,
    |thread_sum, i| *thread_sum += i * i,
    |rx| rx.iter().sum::<i64>()
)
```

Simple parallel implementaion of [word counting](https://benhoyt.com/writings/count-words/):

```rust
use std::{collections::HashMap, io::{stdin, stdout, Read, Write, BufWriter}, cmp::Reverse};

fn main() {
    let mut text = String::new();
    stdin().read_to_string(&mut text).unwrap();
    let lower = text.to_ascii_lowercase();
    let lines: Vec<_> = lower.lines().collect();

    let word_counts = chunker::run(
        &lines,
        chunker::Config { chunk_size: 10_000, ..Default::default() },
        || HashMap::<&str, u32>::new(),
        |counts, line| {
            for word in line.split_whitespace() {
                *counts.entry(word).or_default() += 1;
            }
        },
        |rx| rx.into_iter().reduce(|mut word_counts, counts| {
            for (word, count) in counts {
                *word_counts.entry(word).or_default() += count;
            }

            word_counts
        }).unwrap(),
    );

    let mut sorted_word_counts = Vec::from_iter(word_counts);
    sorted_word_counts.sort_unstable_by_key(|&(_, count)| Reverse(count));
    let mut stdout = BufWriter::new(stdout().lock());

    for (word, count) in sorted_word_counts {
        writeln!(stdout, "{word} {count}").unwrap();
    }
}
```

```
$ hyperfine 'target/release/examples/count_words <kjvbible_x10.txt'
Benchmark 1: target/release/examples/count_words <kjvbible_x10.txt
  Time (mean ± σ):      78.7 ms ±   1.8 ms    [User: 283.2 ms, System: 20.8 ms]
  Range (min … max):    74.9 ms …  84.2 ms    36 runs
```

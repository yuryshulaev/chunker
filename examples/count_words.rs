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

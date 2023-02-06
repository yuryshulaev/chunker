fn main() {
	let mut chunker = chunker::Chunker::new(chunker::Config { progress_bar: false, ..Default::default() });
	let input = vec![1, 2, 3, 4, 5];

	for _ in 0..10_000 {
		let sum = chunker.run(
			&input,
			|| 0,
			|thread_sum, i| *thread_sum += i * i,
			|rx| rx.iter().sum::<i64>(),
		);

		assert_eq!(sum, 55);
	}
}

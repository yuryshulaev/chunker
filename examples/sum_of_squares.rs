fn main() {
	let input = vec![1, 2, 3, 4, 5];

	let sum = chunker::run(
		&input,
		chunker::Config::default(),
		|| 0,
		|thread_sum, i| *thread_sum += i * i,
		|rx| rx.iter().sum::<i64>(),
	);

	assert_eq!(sum, 55);
	println!("{:?}", sum);
}

use std::{thread, sync::{Mutex, mpsc}};

pub struct Config<'a> {
	pub thread_count: usize,
	pub chunk_size: usize,
	#[cfg(feature = "progression")]
	pub bar_step: usize,
	#[cfg(feature = "progression")]
	pub progress_bar: bool,
	#[cfg(feature = "progression")]
	pub bar_config: progression::Config<'a>,
	#[cfg(not(feature = "progression"))]
	pub _phantom: std::marker::PhantomData<&'a ()>,
}

impl Default for Config<'_> {
	#[inline]
	fn default() -> Self {
		Self {
			thread_count: num_cpus::get(),
			chunk_size: 100,
			#[cfg(feature = "progression")]
			bar_step: 10,
			#[cfg(feature = "progression")]
			progress_bar: true,
			#[cfg(feature = "progression")]
			bar_config: progression::Config::default(),
			#[cfg(not(feature = "progression"))]
			_phantom: std::marker::PhantomData,
		}
	}
}

macro_rules! run_template {
	($name:ident, ($($mut:tt)*), $bounds:tt, $chunks_method:ident, $iter_method:ident) => {
		pub fn $name<T, I, R, W, G>(items: &$($mut)* [T], config: Config, init: fn() -> I, work: W, mut gather: G) -> R
		where
			T: $bounds,
			I: Send,
			R: Send,
			W: Fn(&mut I, &$($mut)* T) + Sync,
			G: FnMut(mpsc::Receiver<I>) -> R,
		{
			#[cfg(feature = "progression")]
			let bar = config.progress_bar.then(|| progression::Bar::new(items.len().try_into().unwrap(), config.bar_config));
			#[cfg(feature = "progression")]
			let bar_step = if config.progress_bar { config.bar_step } else { config.chunk_size };
			#[cfg(not(feature = "progression"))]
			let bar_step = config.chunk_size;
			let chunks = Mutex::new(items.$chunks_method(config.chunk_size.min(items.len() / config.thread_count + 1)));

			thread::scope(|scope| {
				let (sender, receiver) = mpsc::channel();

				for _ in 0..config.thread_count {
					let sender = sender.clone();
					let chunks = &chunks;
					let work = &work;
					#[cfg(feature = "progression")]
					let bar = &bar;

					scope.spawn(move || {
						let mut thread_result = init();

						#[allow(clippy::redundant_closure_call)]
						while let Some(chunk) = (|| chunks.lock().unwrap().next())() {
							for bar_chunk in chunk.$chunks_method(bar_step) {
								for item in bar_chunk.$iter_method() {
									work(&mut thread_result, item);
								}

								#[cfg(feature = "progression")]
								if let Some(bar) = bar {
									bar.inc(bar_chunk.len().try_into().unwrap());
								}
							}
						}

						sender.send(thread_result).unwrap()
					});
				}

				drop(sender);
				gather(receiver)
			})
		}
	}
}

run_template!(run, (), Sync, chunks, iter);
run_template!(run_mut, (mut), Send, chunks_mut, iter_mut);

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn it_should_sum_squares() {
		assert_eq!(1 + 4 + 9 + 16 + 25, run(
			&[1, 2, 3, 4, 5],
			Config::default(),
			|| 0,
			|thread_sum, i| *thread_sum += i * i,
			|rx| rx.iter().sum::<i64>(),
		));
	}

	#[test]
	fn it_should_square() {
		let mut items = [1, 2, 3, 4, 5];

		run_mut(
			&mut items,
			Config::default(),
			|| (),
			|_, i| *i *= *i,
			|rx| rx.iter().count(),
		);

		assert_eq!(items, [1, 4, 9, 16, 25]);
	}
}

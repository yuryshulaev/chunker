use std::sync::{Mutex, mpsc};
#[cfg(not(feature = "scoped_threadpool"))]
use std::thread;

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

pub struct Chunker<'a> {
	config: Config<'a>,
	#[cfg(feature = "scoped_threadpool")]
	pool: scoped_threadpool::Pool,
}

macro_rules! run_template {
	($name:ident, ($($mut:tt)*), $bounds:tt, $chunks_method:ident, $iter_method:ident) => {
		impl Chunker<'_> {
			pub fn $name<T, I, R, W, G>(&mut self, items: &$($mut)* [T], init: fn() -> I, work: W, mut gather: G) -> R
			where
				T: $bounds,
				I: Send,
				R: Send,
				W: Fn(&mut I, &$($mut)* T) + Sync,
				G: FnMut(mpsc::Receiver<I>) -> R,
			{
				#[cfg(feature = "progression")]
				let bar = self.config.progress_bar.then(|| progression::Bar::new(items.len().try_into().unwrap(), self.config.bar_config.clone()));
				#[cfg(feature = "progression")]
				let bar_step = if self.config.progress_bar { self.config.bar_step } else { self.config.chunk_size };
				#[cfg(not(feature = "progression"))]
				let bar_step = self.config.chunk_size;
				let chunks = Mutex::new(items.$chunks_method(self.config.chunk_size.min(items.len() / self.config.thread_count + 1)));

				Self::scope(
					#[cfg(feature = "scoped_threadpool")]
					&mut self.pool,
					|scope| {
					let (sender, receiver) = mpsc::channel();

					for _ in 0..self.config.thread_count {
						let sender = sender.clone();
						let chunks = &chunks;
						let work = &work;
						#[cfg(feature = "progression")]
						let bar = &bar;

						let thread_fn = move || {
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
						};

						#[cfg(feature = "scoped_threadpool")]
						scope.execute(thread_fn);
						#[cfg(not(feature = "scoped_threadpool"))]
						scope.spawn(thread_fn);
					}

					drop(sender);
					gather(receiver)
				})
			}
		}

		#[inline]
		pub fn $name<T, I, R, W, G>(items: &$($mut)* [T], config: Config, init: fn() -> I, work: W, gather: G) -> R
		where
			T: $bounds,
			I: Send,
			R: Send,
			W: Fn(&mut I, &$($mut)* T) + Sync,
			G: FnMut(mpsc::Receiver<I>) -> R,
		{
			let mut chunker = Chunker::new(config);
			chunker.$name(items, init, work, gather)
		}
	}
}

impl<'a> Chunker<'a> {
	#[inline]
	pub fn new(config: Config<'a>) -> Self {
		Self {
			#[cfg(feature = "scoped_threadpool")]
			pool: scoped_threadpool::Pool::new(u32::try_from(config.thread_count).unwrap()),
			config,
		}
	}

	#[cfg(feature = "scoped_threadpool")]
	fn scope<'pool, 'scope, F, R>(pool: &'pool mut scoped_threadpool::Pool, f: F) -> R
	where
		F: FnOnce(&scoped_threadpool::Scope<'pool, 'scope>) -> R,
	{
		pool.scoped(f)
	}

	#[cfg(not(feature = "scoped_threadpool"))]
	fn scope<'env, F, T>(f: F) -> T
	where
		F: for<'scope> FnOnce(&'scope std::thread::Scope<'scope, 'env>) -> T,
	{
		thread::scope(f)
	}
}

run_template!(run, (), Sync, chunks, iter);
run_template!(run_mut, (mut), Send, chunks_mut, iter_mut);

impl Default for Chunker<'_> {
	#[inline]
	fn default() -> Self {
		Chunker::new(Config::default())
	}
}

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

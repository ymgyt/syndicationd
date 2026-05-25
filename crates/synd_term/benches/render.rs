use criterion::Criterion;

mod bench {
    use criterion::Criterion;
    use synd_term::{integration, key};

    mod helper;

    pub(super) fn render(c: &mut Criterion) {
        c.bench_function("render", move |b| {
            b.to_async(runtime()).iter_batched(
                || {
                    let app = helper::init_app();
                    let (tx, event_stream) = integration::event_stream();
                    for _ in 0..100 {
                        tx.send(key!('j'));
                    }
                    (app, event_stream)
                },
                |(mut app, mut event_stream)| async move {
                    app.wait_until_jobs_completed(&mut event_stream).await;
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed building the Runtime")
    }
}

pub fn benches() {
    let mut criterion: Criterion<_> = Criterion::default()
        .with_profiler(pprof_flamegraph::Profiler::new(100))
        .configure_from_args();
    bench::render(&mut criterion);
}

fn main() {
    benches();
    criterion::Criterion::default()
        .configure_from_args()
        .final_summary();
}

mod pprof_flamegraph {
    use std::{fs::File, os::raw::c_int, path::Path};

    use criterion::profiler;
    use pprof::ProfilerGuard;

    pub struct Profiler {
        frequency: c_int,
        active_profiler: Option<ProfilerGuard<'static>>,
    }

    impl Profiler {
        pub fn new(frequency: c_int) -> Self {
            Self {
                frequency,
                active_profiler: None,
            }
        }
    }

    impl profiler::Profiler for Profiler {
        fn start_profiling(&mut self, _benchmark_id: &str, _benchmark_dir: &Path) {
            self.active_profiler = Some(ProfilerGuard::new(self.frequency).unwrap());
        }

        fn stop_profiling(&mut self, _benchmark_id: &str, benchmark_dir: &Path) {
            std::fs::create_dir_all(benchmark_dir).unwrap();
            let output_path = benchmark_dir.join("flamegraph.svg");
            let output_file = File::create(&output_path).unwrap_or_else(|_| {
                panic!("File system error while creating {}", output_path.display())
            });

            if let Some(profiler) = self.active_profiler.take() {
                profiler
                    .report()
                    .build()
                    .unwrap()
                    .flamegraph(output_file)
                    .expect("Error while writing flamegraph");
            }
        }
    }
}

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

// Explicitly using the expanded code of the following lines.
// criterion::criterion_group!(benches, bench::render);
// criterion::criterion_main!(benches);
pub fn benches() {
    let mut criterion: criterion::Criterion<_> =
        criterion::Criterion::default().configure_from_args();
    bench::render(&mut criterion);
}

fn main() {
    benches();
    criterion::Criterion::default()
        .configure_from_args()
        .final_summary();
}

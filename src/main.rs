use plotters::prelude::*;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};

fn main() {
    let sample_size = 10;

    let thread_counts = vec![10, 25, 50, 100, 200, 400, 800];

    // a lambda to run a locking experiment many times across different thread counts and
    // accumulates the results
    let collect_samples = |func: fn(&AtomicBool)| {
        thread_counts
            .iter()
            .map(|thread_count| {
                let samples = (0..sample_size)
                    .map(|_| run_experiment(func, *thread_count))
                    .collect();
                println!("Experiment complete for {}", thread_count);
                (*thread_count, samples)
            })
            .collect()
    };

    let experiments1 = collect_samples(ttas);
    let experiments2 = collect_samples(tas);

    line_plot(experiments1, experiments2);
}

/// plots a scatter plot of run duration vs thread count
fn line_plot(durations1: Vec<(usize, Vec<Duration>)>, durations2: Vec<(usize, Vec<Duration>)>) {
    // closure to turn raw duration results into data for plotting
    let make_data = |durations: Vec<(usize, Vec<Duration>)>| {
        let mut data: Vec<(f64, f64)> = vec![];
        durations.iter().for_each(|(thread_count, samples)| {
            samples
                .iter()
                .for_each(|sample| data.push((*thread_count as f64, sample.as_nanos() as f64)))
        });
        data
    };

    // turn raw data into plottable data
    let data1 = make_data(durations1);
    let data2 = make_data(durations2);

    let out_file_name = "plots/ttas.png";

    let root = BitMapBackend::new(out_file_name, (1024, 768)).into_drawing_area();
    root.fill(&WHITE).unwrap();

    let areas = root.split_by_breakpoints([944], [80]);

    let mut scatter_ctx = ChartBuilder::on(&areas[2])
        .caption(
            "Execution time vs. thread count",
            ("sans-serif", 50).into_font(),
        )
        .x_label_area_size(80)
        .y_label_area_size(80)
        .build_cartesian_2d(0f64..1000f64, (0f64..20_000_000f64).log_scale())
        .unwrap();
    scatter_ctx
        .configure_mesh()
        .disable_x_mesh()
        .disable_y_mesh()
        .y_desc("Execution time (ns)")
        .x_desc("Thread count")
        .draw()
        .unwrap();

    scatter_ctx
        .draw_series(
            data1
                .iter()
                .map(|(x, y)| Circle::new((*x, *y), 3, GREEN.stroke_width(1))),
        )
        .unwrap()
        .label("y = x^2")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

    scatter_ctx
        .draw_series(
            data2
                .iter()
                .map(|(x, y)| Circle::new((*x, *y), 3, RED.stroke_width(1))),
        )
        .unwrap()
        .label("y = x^2")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

    // To avoid the IO failure being ignored silently, we manually call the present function
    root.present().expect(
        format!(
            "Unable to write result to file, please make sure '{}' dir exists under current dir",
            out_file_name
        )
        .as_str(),
    );
    println!("Result has been saved to {}", out_file_name);
}

/// runs a lock function on a given number of threads and returns the duration
fn run_experiment(func: fn(&AtomicBool), thread_count: usize) -> Duration {
    let lock = AtomicBool::new(false);
    let lock: &'static _ = Box::leak(Box::new(lock));

    let barrier = Arc::new(Barrier::new(thread_count + 1));

    let threads: Vec<_> = (0..thread_count)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            let lock = lock;
            std::thread::spawn(move || {
                barrier.wait();
                func(lock);
            })
        })
        .collect();

    barrier.wait();

    let now = Instant::now();

    threads.into_iter().for_each(|t| t.join().unwrap());

    let elapsed = now.elapsed();

    return elapsed;
}

fn ttas(lock: &AtomicBool) {
    // acquire lock
    loop {
        while lock.load(std::sync::atomic::Ordering::Acquire) {}

        if let Ok(_) = lock.compare_exchange(
            false,
            true,
            std::sync::atomic::Ordering::SeqCst,
            std::sync::atomic::Ordering::SeqCst,
        ) {
            break;
        }
    }

    // ... do work

    // release lock
    lock.store(false, std::sync::atomic::Ordering::SeqCst);
}

fn tas(lock: &AtomicBool) {
    // acquire lock
    loop {
        if let Ok(_) = lock.compare_exchange(
            false,
            true,
            std::sync::atomic::Ordering::SeqCst,
            std::sync::atomic::Ordering::SeqCst,
        ) {
            break;
        }
    }

    // ... do work

    // release lock
    lock.store(false, std::sync::atomic::Ordering::SeqCst);
}

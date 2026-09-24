//! Native release measurements; excludes Typst, JSON transport and rendering.
use cetz_nurbs::construct::{PointInput, from_controls, from_interpolation};
use cetz_nurbs::display::native_cubics;
use cetz_nurbs::nurbs::NurbsInput;
use std::{hint::black_box, time::Instant};

fn measure(name: &str, mut operation: impl FnMut() -> usize) {
    let count = operation();
    let mut samples = Vec::new();
    for _ in 0..5 {
        let started = Instant::now();
        let mut calls = 0;
        loop {
            black_box(operation());
            calls += 1;
            if started.elapsed().as_millis() >= 100 {
                break;
            }
        }
        samples.push(started.elapsed().as_secs_f64() * 1e6 / calls as f64);
    }
    samples.sort_by(f64::total_cmp);
    println!(
        "{}",
        serde_json::json!({"case":name,"output_count":count,
        "median_us":samples[2],"min_us":samples[0],"max_us":samples[4]})
    );
}

fn input(n: usize, degree: usize, periodic: bool) -> PointInput {
    PointInput {
        points: (0..n)
            .map(|i| {
                let t = i as f64 / (n - 1) as f64;
                vec![10. * t, (t * 4. * std::f64::consts::PI).sin(), 0.]
            })
            .collect(),
        degree,
        close: false,
        periodic,
    }
}

fn main() {
    for n in [8, 32, 128, 512, 1024] {
        let points = input(n, 3, false);
        let spec: NurbsInput =
            serde_json::from_value(serde_json::to_value(from_controls(&points).unwrap()).unwrap())
                .unwrap();
        measure(&format!("poly3_n{n}"), || {
            native_cubics(black_box(&spec)).unwrap().len()
        });
    }
    for degree in [3, 5, 8] {
        let mut data = from_controls(&input(32, degree, false)).unwrap();
        data.weights = (0..32).map(|i| 1. + 0.6 * (i as f64 * 1.7).sin()).collect();
        for tolerance in [0.01, 0.0001, 0.000001] {
            let mut value = serde_json::to_value(&data).unwrap();
            value["tolerance"] = tolerance.into();
            let spec: NurbsInput = serde_json::from_value(value).unwrap();
            measure(&format!("rational_p{degree}_tol{tolerance}"), || {
                native_cubics(black_box(&spec)).unwrap().len()
            });
        }
    }
    for periodic in [false, true] {
        for n in [16, 64, 128, 256, 512] {
            let points = input(n, 3, periodic);
            measure(&format!("interpolate_n{n}_periodic{periodic}"), || {
                from_interpolation(black_box(&points))
                    .unwrap()
                    .control_points
                    .len()
            });
        }
    }
}

use std::fmt::{Debug, Formatter};
use std::time::{Duration, Instant};

pub struct Profiler {
    name: String,
    vars: Vec<String>,
    epoch: Instant,
    every: Duration,
    t0: Vec<Instant>,
    dt: Vec<Vec<Duration>>,
}

impl Profiler {
    pub fn new(name: impl Into<String>, vars: Vec<impl Into<String>>, every: Duration) -> Self {
        let time = Instant::now();
        let n_vars = vars.len();

        Self {
            name: name.into(),
            vars: vars.into_iter().map(|a| a.into()).collect(),
            epoch: time,
            every,
            t0: vec![time; n_vars],
            dt: vec![Vec::with_capacity(10_000); n_vars],
        }
    }

    pub fn tic(&mut self, i: usize) {
        self.t0[i] = Instant::now();
    }

    pub fn toc(&mut self, i: usize) {
        self.dt[i].push(Instant::now().duration_since(self.t0[i]));
    }

    pub fn step(&mut self) {
        let time = Instant::now();
        if time.duration_since(self.epoch) > self.every {
            self.epoch = time;
            self.report();
            self.reset();
        }
    }

    pub fn report(&mut self) {
        println!("Profiler report: {}", self.name);
        self.dt.iter_mut().enumerate().for_each(|(i, v)| {
            if v.is_empty() {
                return;
            }

            v.sort_by(|a, b| a.partial_cmp(b).unwrap());

            println!(
                "    {:4}: {}; {}; {}; {}; {}; {}; {}; {}",
                self.vars[i],
                Self::format("avg", v.iter().sum::<Duration>().div_f32(v.len() as f32)),
                Self::format("min", v[0]),
                Self::format("q01", v[v.len() * 1 / 100]),
                Self::format("q01", v[v.len() * 10 / 100]),
                Self::format("q10", v[v.len() * 50 / 100]),
                Self::format("q50", v[v.len() * 90 / 100]),
                Self::format("q90", v[v.len() * 99 / 100]),
                Self::format("max", v[v.len() - 1]),
            );
        });
        println!("---------")
    }

    pub fn reset(&mut self) {
        self.epoch = Instant::now();
        self.dt.iter_mut().enumerate().for_each(|(_, v)| {
            v.clear();
        });
    }

    fn format(label: &str, x: Duration) -> String {
        format!("{label}={:5.02}ms", x.as_nanos() as f64 / 1_000_000.0)
    }
}

impl Debug for Profiler {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Profiler(name={}, vars={:?}, epoch={:?} ago, every={:?})",
            self.name,
            self.vars,
            self.epoch.elapsed(),
            self.every
        )
    }
}

use std::time::{Duration, Instant};

pub struct PhaseTimings {
    last: Instant,
    phases: Vec<(&'static str, Duration)>,
}

impl PhaseTimings {
    pub fn start() -> Self {
        Self {
            last: Instant::now(),
            phases: Vec::new(),
        }
    }

    pub fn mark(&mut self, label: &'static str) {
        let now = Instant::now();
        self.phases.push((label, now.duration_since(self.last)));
        self.last = now;
    }

    pub fn print_stderr(&self) {
        for (label, duration) in &self.phases {
            eprintln!("{label}: {:.3}ms", duration.as_secs_f64() * 1000.0);
        }
    }
}

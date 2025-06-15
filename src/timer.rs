/// A timer that logs the elapsed time when it is dropped.
pub struct ElapsedTimer {
    name: String,
    start: std::time::Instant,
}

impl ElapsedTimer {
    /// Creates a new `ElapsedTimer` instance that starts timing immediately.
    pub fn new(name: &str) -> Self {
        let timer = Self {
            name: name.to_owned(),
            start: std::time::Instant::now(),
        };
        log::info!("START {}", timer.name);
        timer
    }
}

impl Drop for ElapsedTimer {
    fn drop(&mut self) {
        let end = std::time::Instant::now();
        let elapsed = end - self.start;
        log::info!("ENDED {} Elapsed time: {}", self.name, elapsed.as_millis());
    }
}

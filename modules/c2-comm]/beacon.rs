use std::time::{Duration, Instant};
use rand::Rng;

pub struct BeaconController {
    interval: Duration,
    jitter: Duration,
    last_beacon: Instant,
    failed_attempts: u32,
    max_failures: u32,
    backoff_multiplier: f64,
}

impl BeaconController {
    pub fn new(interval_secs: u64, jitter_secs: u64) -> Self {
        Self {
            interval: Duration::from_secs(interval_secs),
            jitter: Duration::from_secs(jitter_secs),
            last_beacon: Instant::now() - Duration::from_secs(interval_secs), // Start immediately
            failed_attempts: 0,
            max_failures: 5,
            backoff_multiplier: 2.0,
        }
    }
    
    pub fn time_until_next_beacon(&self) -> Duration {
        let elapsed = self.last_beacon.elapsed();
        let target_interval = self.calculate_interval();
        
        if elapsed >= target_interval {
            Duration::from_secs(0)
        } else {
            target_interval - elapsed
        }
    }
    
    pub fn should_beacon(&self) -> bool {
        self.time_until_next_beacon().as_secs() == 0
    }
    
    pub fn record_success(&mut self) {
        self.failed_attempts = 0;
        self.last_beacon = Instant::now();
    }
    
    pub fn record_failure(&mut self) {
        self.failed_attempts += 1;
        self.last_beacon = Instant::now();
    }
    
    pub fn is_max_failures_reached(&self) -> bool {
        self.failed_attempts >= self.max_failures
    }
    
    fn calculate_interval(&self) -> Duration {
        let base_interval = if self.failed_attempts > 0 {
            // Exponential backoff on failures
            let multiplier = self.backoff_multiplier.powi(self.failed_attempts as i32);
            self.interval.mul_f64(multiplier.min(3600.0)) // Cap at 1 hour
        } else {
            self.interval
        };
        
        // Add jitter
        let jitter_ms = rand::thread_rng()
            .gen_range(0..self.jitter.as_millis() as u64);
        
        base_interval + Duration::from_millis(jitter_ms)
    }
    
    pub fn update_interval(&mut self, new_interval: u64) {
        self.interval = Duration::from_secs(new_interval);
    }
    
    pub fn sleep_with_interrupt(&self, check_interval: Duration, should_stop: &std::sync::atomic::AtomicBool) -> bool {
        let start = Instant::now();
        let total = self.time_until_next_beacon();
        
        while start.elapsed() < total {
            if should_stop.load(std::sync::atomic::Ordering::Relaxed) {
                return false;
            }
            std::thread::sleep(check_interval.min(total - start.elapsed()));
        }
        
        true
    }
}
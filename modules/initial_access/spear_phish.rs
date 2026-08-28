use std::collections::HashMap;
use serde::{Serialize, Deserialize};

pub struct SpearPhisher {
    template: EmailTemplate,
    targets: Vec<Target>,
    attachment_path: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EmailTemplate {
    pub subject: String,
    pub body: String,
    pub from_name: String,
    pub from_email: String,
}

#[derive(Clone)]
pub struct Target {
    pub email: String,
    pub name: String,
    pub department: String,
    pub role: String,
}

impl SpearPhisher {
    pub fn new(template: EmailTemplate, attachment: &str) -> Self {
        Self {
            template,
            targets: Vec::new(),
            attachment_path: attachment.to_string(),
        }
    }
    
    pub fn load_targets_from_csv(&mut self, csv_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(csv_path)?;
        
        for line in content.lines().skip(1) { // Skip header
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 4 {
                self.targets.push(Target {
                    email: parts[0].to_string(),
                    name: parts[1].to_string(),
                    department: parts[2].to_string(),
                    role: parts[3].to_string(),
                });
            }
        }
        
        Ok(())
    }
    
    pub fn personalize_template(&self, target: &Target) -> EmailTemplate {
        let mut personalized = self.template.clone();
        
        // Replace placeholders
        personalized.subject = personalized.subject
            .replace("{name}", &target.name)
            .replace("{department}", &target.department)
            .replace("{role}", &target.role);
        
        personalized.body = personalized.body
            .replace("{name}", &target.name)
            .replace("{department}", &target.department)
            .replace("{role}", &target.role)
            .replace("{email}", &target.email);
        
        personalized
    }
    
    pub fn generate_templates(&self) -> HashMap<String, EmailTemplate> {
        let mut templates = HashMap::new();
        
        for target in &self.targets {
            let personalized = self.personalize_template(target);
            templates.insert(target.email.clone(), personalized);
        }
        
        templates
    }
    
    pub fn get_default_academic_template() -> EmailTemplate {
        EmailTemplate {
            subject: "Important: Updated Schedule for {department} Department".to_string(),
            from_name: "IT Support".to_string(),
            from_email: "it-support@college.edu".to_string(),
            body: r#"Dear {name},

Due to recent system updates, we have revised the schedule for the {department} department. 

Please review the attached document for your updated timetable and room assignments.

If you have any questions, please contact the IT Help Desk.

Best regards,
IT Support Team
College IT Department"#.to_string(),
        }
    }
    
    pub fn get_urgent_template() -> EmailTemplate {
        EmailTemplate {
            subject: "URGENT: Action Required - Account Verification".to_string(),
            from_name: "Security Team".to_string(),
            from_email: "security@college.edu".to_string(),
            body: r#"Dear {name},

Our system has detected unusual activity on your account. Please verify your identity by opening the attached secure document.

Failure to respond within 24 hours will result in account suspension.

Security Team"#.to_string(),
        }
    }
}
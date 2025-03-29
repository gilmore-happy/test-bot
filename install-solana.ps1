# PowerShell script to install Solana CLI on Windows
Write-Host "Downloading Solana installer..."

# Create a temporary directory
$tempDir = [System.IO.Path]::GetTempPath() + [System.Guid]::NewGuid().ToString()
New-Item -ItemType Directory -Path $tempDir | Out-Null

# Download the Solana installer
$installerUrl = "https://release.solana.com/v1.17.7/solana-install-init-x86_64-pc-windows-msvc.exe"
$installerPath = Join-Path $tempDir "solana-installer.exe"

try {
    Invoke-WebRequest -Uri $installerUrl -OutFile $installerPath
    Write-Host "Download completed successfully."
    
    # Run the installer
    Write-Host "Running Solana installer..."
    Start-Process -FilePath $installerPath -Wait
    
    # Add Solana to the PATH
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path", "User") + ";" + $HOME + "\.local\share\solana\install\active_release\bin"
    [System.Environment]::SetEnvironmentVariable("Path", $env:Path, "User")
    
    Write-Host "Solana CLI has been installed. Please restart your terminal to use the 'solana' command."
} catch {
    Write-Host "Error: $_"
}

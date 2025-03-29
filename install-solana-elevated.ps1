# PowerShell script to install Solana CLI with elevated privileges

# Self-elevate if not already running as administrator
if (-Not ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole] 'Administrator')) {
    Write-Host "Requesting administrator privileges..."
    Start-Process PowerShell.exe -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`"" -Verb RunAs
    exit
}

Write-Host "Installing Solana CLI with administrator privileges..."

# Create a temporary directory
$tempDir = "$env:TEMP\solana-install"
if (Test-Path $tempDir) {
    Remove-Item -Path $tempDir -Recurse -Force
}
New-Item -ItemType Directory -Path $tempDir | Out-Null
Set-Location $tempDir

# Download the installer
Write-Host "Downloading Solana installer..."
$installerUrl = "https://release.solana.com/v1.17.7/solana-install-init-x86_64-pc-windows-msvc.exe"
$installerPath = "$tempDir\solana-install-init.exe"
Invoke-WebRequest -Uri $installerUrl -OutFile $installerPath

# Run the installer
Write-Host "Running Solana installer..."
Start-Process -FilePath $installerPath -ArgumentList "v1.17.7" -Wait

# Add Solana to the PATH for this session
$env:PATH = "$env:USERPROFILE\.local\share\solana\install\active_release\bin;$env:PATH"

# Verify installation
Write-Host "Verifying installation..."
try {
    $version = & solana --version
    Write-Host $version
    Write-Host "Installation complete."
} catch {
    Write-Host "Failed to verify installation. Please restart your terminal and try 'solana --version'."
}

# Add Solana to the user's PATH permanently
$userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
$solanaPath = "$env:USERPROFILE\.local\share\solana\install\active_release\bin"
if (-not $userPath.Contains($solanaPath)) {
    [Environment]::SetEnvironmentVariable("PATH", "$userPath;$solanaPath", "User")
    Write-Host "Added Solana to your PATH environment variable."
}

Write-Host "Press any key to continue..."
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")

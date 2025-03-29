@echo off
echo Installing Solana CLI with administrator privileges...

:: Check for admin rights
net session >nul 2>&1
if %errorLevel% neq 0 (
    echo This script requires administrator privileges.
    echo Right-click on the file and select "Run as administrator".
    pause
    exit /b 1
)

:: Create a temporary directory
set TEMP_DIR=%TEMP%\solana-install
if exist %TEMP_DIR% rmdir /s /q %TEMP_DIR%
mkdir %TEMP_DIR%
cd %TEMP_DIR%

:: Download the installer
echo Downloading Solana installer...
curl -L -o solana-install-init.exe https://release.solana.com/v1.17.7/solana-install-init-x86_64-pc-windows-msvc.exe

:: Run the installer
echo Running Solana installer...
solana-install-init.exe v1.17.7

:: Add Solana to the PATH for this session
set PATH=%USERPROFILE%\.local\share\solana\install\active_release\bin;%PATH%

:: Verify installation
echo Verifying installation...
solana --version

echo Installation complete. Please restart your terminal to use the Solana CLI.
pause

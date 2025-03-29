@echo off
REM Installation script for Solana HFT Bot on Windows

echo Installing Solana HFT Bot...

REM Check if Rust is installed
where rustc >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo Rust is not installed. Installing Rust...
    curl --proto '=https' --tlsv1.2 -sSf https://win.rustup.rs/x86_64 -o rustup-init.exe
    rustup-init.exe -y
    del rustup-init.exe
    set PATH=%USERPROFILE%\.cargo\bin;%PATH%
) else (
    echo Rust is already installed.
)

REM Install nightly for some features
call rustup toolchain install nightly
call rustup component add rust-src --toolchain nightly

REM Check if Solana CLI is installed
where solana >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo Solana CLI is not installed. Installing Solana CLI...
    curl -sSfL https://release.solana.com/v1.17.22/solana-install-init-x86_64-pc-windows-msvc.exe -o solana-install-init.exe
    .\solana-install-init.exe v1.17.22
    del solana-install-init.exe
    set PATH=%LOCALAPPDATA%\solana\install\active_release\bin;%PATH%
) else (
    echo Solana CLI is already installed.
)

REM Create config directory
if not exist config mkdir config

REM Create default config file if it doesn't exist
if not exist config\default.toml (
    echo Creating default configuration file...
    (
        echo [network]
        echo use_dpdk = false
        echo use_io_uring = false
        echo worker_threads = 4
        echo.
        echo [rpc]
        echo endpoints = [
        echo     "https://api.mainnet-beta.solana.com",
        echo     "https://solana-api.projectserum.com"
        echo ]
        echo connection_pool_size = 5
        echo.
        echo [execution]
        echo use_jito = false
        echo simulate_transactions = true
        echo.
        echo [arbitrage]
        echo min_profit_threshold = 0.001
        echo max_position_size = 1000
    ) > config\default.toml
)

REM Note about DPDK and io_uring
echo Note: DPDK and io_uring are primarily Linux features and are disabled on Windows.
echo For optimal performance, consider running on a Linux environment.

REM Build the project
echo Building Solana HFT Bot...
call cargo build --release

echo Installation complete!
echo To run the bot, use: cargo run --release -- --config config\default.toml

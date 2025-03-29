2  @echo off
REM Create project structure
mkdir solana-hft-bot
cd solana-hft-bot

REM Initialize workspace
echo [workspace] > Cargo.toml
echo resolver = "2" >> Cargo.toml
echo members = [ >> Cargo.toml
echo     "crates/core", >> Cargo.toml
echo     "crates/network", >> Cargo.toml
echo     "crates/execution", >> Cargo.toml
echo     "crates/arbitrage", >> Cargo.toml
echo     "crates/monitoring", >> Cargo.toml
echo     "crates/screening", >> Cargo.toml
echo     "crates/risk", >> Cargo.toml
echo     "crates/rpc", >> Cargo.toml
echo     "crates/security", >> Cargo.toml
echo     "crates/metrics", >> Cargo.toml
echo     "crates/cli" >> Cargo.toml
echo ] >> Cargo.toml
echo. >> Cargo.toml
echo [workspace.dependencies] >> Cargo.toml
echo # Async runtime >> Cargo.toml
echo tokio = { version = "1.36.0", features = ["full", "rt-multi-thread"] } >> Cargo.toml
echo tokio-util = { version = "0.7.10", features = ["full"] } >> Cargo.toml
echo futures = "0.3.30" >> Cargo.toml
echo. >> Cargo.toml
echo # Solana client libraries >> Cargo.toml
echo solana-client = "1.17.22" >> Cargo.toml
echo solana-sdk = "1.17.22" >> Cargo.toml
echo solana-transaction-status = "1.17.22" >> Cargo.toml
echo solana-program = "1.17.22" >> Cargo.toml
echo solana-account-decoder = "1.17.22" >> Cargo.toml
echo solana-rpc = "1.17.22" >> Cargo.toml
echo. >> Cargo.toml
echo # Jito MEV libraries >> Cargo.toml
echo jito-tip-distribution = "0.5.0" >> Cargo.toml
echo jito-bundle = "0.3.0" >> Cargo.toml
echo jito-searcher-client = "0.3.0" >> Cargo.toml
echo. >> Cargo.toml
echo # Network optimizations >> Cargo.toml
echo io-uring = "0.6.3" >> Cargo.toml
echo socket2 = { version = "0.5.5", features = ["all"] } >> Cargo.toml
echo dpdk-sys = "23.11.0" >> Cargo.toml
echo etherparse = "0.13.0" >> Cargo.toml
echo mio = "0.8.10" >> Cargo.toml
echo. >> Cargo.toml
echo # Cryptography >> Cargo.toml
echo ed25519-dalek = "2.1.1" >> Cargo.toml
echo curve25519-dalek = { version = "4.1.1", features = ["digest", "legacy_compatibility"] } >> Cargo.toml
echo ring = "0.17.7" >> Cargo.toml
echo sha2 = "0.10.8" >> Cargo.toml
echo hmac = "0.12.1" >> Cargo.toml
echo rand = "0.8.5" >> Cargo.toml
echo zeroize = "1.7.0" >> Cargo.toml
echo. >> Cargo.toml
echo # Serialization >> Cargo.toml
echo serde = { version = "1.0.197", features = ["derive"] } >> Cargo.toml
echo serde_json = "1.0.114" >> Cargo.toml
echo bincode = "1.3.3" >> Cargo.toml
echo borsh = "1.3.1" >> Cargo.toml
echo. >> Cargo.toml
echo # Logging and metrics >> Cargo.toml
echo tracing = "0.1.40" >> Cargo.toml
echo tracing-subscriber = { version = "0.3.18", features = ["env-filter"] } >> Cargo.toml
echo prometheus = "0.13.3" >> Cargo.toml
echo prometheus-static-metric = "0.5.1" >> Cargo.toml
echo. >> Cargo.toml
echo # Error handling >> Cargo.toml
echo thiserror = "1.0.56" >> Cargo.toml
echo anyhow = "1.0.79" >> Cargo.toml
echo. >> Cargo.toml
echo # Utilities >> Cargo.toml
echo dashmap = "5.5.3" >> Cargo.toml
echo parking_lot = "0.12.1" >> Cargo.toml
echo rayon = "1.8.1" >> Cargo.toml
echo hashbrown = "0.14.3" >> Cargo.toml
echo crossbeam = "0.8.4" >> Cargo.toml
echo itertools = "0.12.1" >> Cargo.toml
echo num_cpus = "1.16.0" >> Cargo.toml
echo chrono = { version = "0.4.33", features = ["serde"] } >> Cargo.toml
echo async-trait = "0.1.77" >> Cargo.toml
echo config = "0.13.4" >> Cargo.toml
echo once_cell = "1.19.0" >> Cargo.toml
echo bytes = "1.5.0" >> Cargo.toml
echo smallvec = { version = "1.13.1", features = ["const_generics", "union", "write"] } >> Cargo.toml
echo. >> Cargo.toml
echo [workspace.lints.rust] >> Cargo.toml
echo unsafe_code = "allow"  # Needed for performance optimizations >> Cargo.toml
echo. >> Cargo.toml
echo [profile.release] >> Cargo.toml
echo opt-level = 3 >> Cargo.toml
echo lto = "fat" >> Cargo.toml
echo codegen-units = 1 >> Cargo.toml
echo panic = "abort" >> Cargo.toml
echo debug = true  # Keep debug symbols for profiling >> Cargo.toml
echo strip = false >> Cargo.toml
echo. >> Cargo.toml
echo [profile.dev] >> Cargo.toml
echo opt-level = 0 >> Cargo.toml
echo debug = true >> Cargo.toml
echo. >> Cargo.toml
echo # Profile for benchmarking and testing with some optimizations >> Cargo.toml
echo [profile.bench] >> Cargo.toml
echo opt-level = 3 >> Cargo.toml
echo debug = true >> Cargo.toml

REM Create crates directories
mkdir crates\core crates\network crates\execution crates\arbitrage crates\monitoring crates\screening crates\risk crates\rpc crates\security crates\metrics crates\cli

REM Initialize each crate
for %%c in (core network execution arbitrage monitoring screening risk rpc security metrics cli) do (
    mkdir crates\%%c\src
    
    REM Create Cargo.toml for each crate
    echo [package] > crates\%%c\Cargo.toml
    echo name = "solana-hft-%%c" >> crates\%%c\Cargo.toml
    echo version = "0.1.0" >> crates\%%c\Cargo.toml
    echo edition = "2021" >> crates\%%c\Cargo.toml
    echo description = "Solana HFT Bot - %%c module" >> crates\%%c\Cargo.toml
    echo authors = ["Your Name ^<your.email@example.com^>"] >> crates\%%c\Cargo.toml
    echo repository = "https://github.com/yourusername/solana-hft-bot" >> crates\%%c\Cargo.toml
    echo license = "MIT OR Apache-2.0" >> crates\%%c\Cargo.toml
    echo. >> crates\%%c\Cargo.toml
    echo [dependencies] >> crates\%%c\Cargo.toml
    echo # Workspace dependencies >> crates\%%c\Cargo.toml
    echo tokio = { workspace = true } >> crates\%%c\Cargo.toml
    echo futures = { workspace = true } >> crates\%%c\Cargo.toml
    echo solana-client = { workspace = true } >> crates\%%c\Cargo.toml
    echo solana-sdk = { workspace = true } >> crates\%%c\Cargo.toml
    echo serde = { workspace = true } >> crates\%%c\Cargo.toml
    echo serde_json = { workspace = true } >> crates\%%c\Cargo.toml
    echo tracing = { workspace = true } >> crates\%%c\Cargo.toml
    echo thiserror = { workspace = true } >> crates\%%c\Cargo.toml
    echo anyhow = { workspace = true } >> crates\%%c\Cargo.toml
    echo. >> crates\%%c\Cargo.toml
    echo # Add crate-specific dependencies here >> crates\%%c\Cargo.toml
    echo. >> crates\%%c\Cargo.toml
    echo [lints] >> crates\%%c\Cargo.toml
    echo workspace = true >> crates\%%c\Cargo.toml
    echo. >> crates\%%c\Cargo.toml
    echo [features] >> crates\%%c\Cargo.toml
    echo default = [] >> crates\%%c\Cargo.toml
    echo simd = []  # Enable SIMD optimizations >> crates\%%c\Cargo.toml
    
    REM Create lib.rs
    echo //! Solana HFT Bot - %%c module > crates\%%c\src\lib.rs
    echo //! >> crates\%%c\src\lib.rs
    echo //! This module provides functionality for the %%c component of the Solana HFT Bot. >> crates\%%c\src\lib.rs
    echo. >> crates\%%c\src\lib.rs
    echo #![allow(dead_code)] >> crates\%%c\src\lib.rs
    echo #![allow(unused_variables)] >> crates\%%c\src\lib.rs
    echo #![cfg_attr(feature = "simd", feature(stdsimd))] >> crates\%%c\src\lib.rs
    echo. >> crates\%%c\src\lib.rs
    echo use tracing::{debug, error, info, trace, warn}; >> crates\%%c\src\lib.rs
    echo. >> crates\%%c\src\lib.rs
    echo pub fn init() { >> crates\%%c\src\lib.rs
    echo     info!("Initializing %%c module"); >> crates\%%c\src\lib.rs
    echo } >> crates\%%c\src\lib.rs
    echo. >> crates\%%c\src\lib.rs
    echo #[cfg(test)] >> crates\%%c\src\lib.rs
    echo mod tests { >> crates\%%c\src\lib.rs
    echo     use super::*; >> crates\%%c\src\lib.rs
    echo. >> crates\%%c\src\lib.rs
    echo     #[test] >> crates\%%c\src\lib.rs
    echo     fn it_works() { >> crates\%%c\src\lib.rs
    echo         assert_eq!(2 + 2, 4); >> crates\%%c\src\lib.rs
    echo     } >> crates\%%c\src\lib.rs
    echo } >> crates\%%c\src\lib.rs
)

REM Create main.rs for CLI
echo //! Solana HFT Bot - Command Line Interface > crates\cli\src\main.rs
echo //! >> crates\cli\src\main.rs
echo //! This is the main entry point for the Solana HFT Bot. >> crates\cli\src\main.rs
echo. >> crates\cli\src\main.rs
echo use anyhow::Result; >> crates\cli\src\main.rs
echo use tracing::{debug, error, info, trace, warn}; >> crates\cli\src\main.rs
echo use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter}; >> crates\cli\src\main.rs
echo. >> crates\cli\src\main.rs
echo fn main() -^> Result^<()^> { >> crates\cli\src\main.rs
echo     // Initialize logging >> crates\cli\src\main.rs
echo     tracing_subscriber::registry() >> crates\cli\src\main.rs
echo         .with(EnvFilter::from_default_env()) >> crates\cli\src\main.rs
echo         .with(tracing_subscriber::fmt::layer()) >> crates\cli\src\main.rs
echo         .init(); >> crates\cli\src\main.rs
echo. >> crates\cli\src\main.rs
echo     info!("Solana HFT Bot starting up"); >> crates\cli\src\main.rs
echo     >> crates\cli\src\main.rs
echo     // Initialize modules >> crates\cli\src\main.rs
echo     solana_hft_core::init(); >> crates\cli\src\main.rs
echo     solana_hft_network::init(); >> crates\cli\src\main.rs
echo     solana_hft_execution::init(); >> crates\cli\src\main.rs
echo     solana_hft_arbitrage::init(); >> crates\cli\src\main.rs
echo     solana_hft_monitoring::init(); >> crates\cli\src\main.rs
echo     solana_hft_screening::init(); >> crates\cli\src\main.rs
echo     solana_hft_risk::init(); >> crates\cli\src\main.rs
echo     solana_hft_rpc::init(); >> crates\cli\src\main.rs
echo     solana_hft_security::init(); >> crates\cli\src\main.rs
echo     solana_hft_metrics::init(); >> crates\cli\src\main.rs
echo     >> crates\cli\src\main.rs
echo     info!("Solana HFT Bot initialization complete"); >> crates\cli\src\main.rs
echo     >> crates\cli\src\main.rs
echo     // TODO: Implement main logic >> crates\cli\src\main.rs
echo     >> crates\cli\src\main.rs
echo     Ok(()) >> crates\cli\src\main.rs
echo } >> crates\cli\src\main.rs

REM Add dependencies to CLI crate
echo. >> crates\cli\Cargo.toml
echo # Add dependencies to other crates in the workspace >> crates\cli\Cargo.toml
echo solana-hft-core = { path = "../core" } >> crates\cli\Cargo.toml
echo solana-hft-network = { path = "../network" } >> crates\cli\Cargo.toml
echo solana-hft-execution = { path = "../execution" } >> crates\cli\Cargo.toml
echo solana-hft-arbitrage = { path = "../arbitrage" } >> crates\cli\Cargo.toml
echo solana-hft-monitoring = { path = "../monitoring" } >> crates\cli\Cargo.toml
echo solana-hft-screening = { path = "../screening" } >> crates\cli\Cargo.toml
echo solana-hft-risk = { path = "../risk" } >> crates\cli\Cargo.toml
echo solana-hft-rpc = { path = "../rpc" } >> crates\cli\Cargo.toml
echo solana-hft-security = { path = "../security" } >> crates\cli\Cargo.toml
echo solana-hft-metrics = { path = "../metrics" } >> crates\cli\Cargo.toml
echo. >> crates\cli\Cargo.toml
echo [[bin]] >> crates\cli\Cargo.toml
echo name = "solana-hft-bot" >> crates\cli\Cargo.toml
echo path = "src/main.rs" >> crates\cli\Cargo.toml

REM Create a README.md
echo # Solana High-Frequency Trading Bot > README.md
echo. >> README.md
echo A high-performance, low-latency trading bot for Solana blockchain, focusing on sniping, arbitrage, and MEV opportunities. >> README.md
echo. >> README.md
echo ## Features >> README.md
echo. >> README.md
echo - High-frequency trading with ^<100 microsecond latency >> README.md
echo - Kernel-bypass networking using DPDK/io_uring >> README.md
echo - SIMD optimizations for critical path operations >> README.md
echo - Real-time monitoring and token screening >> README.md
echo - Advanced trade execution with pre-signed transaction vaults >> README.md
echo - Multi-DEX arbitrage >> README.md
echo - Comprehensive risk management >> README.md
echo - Production-grade security and monitoring >> README.md
echo. >> README.md
echo ## System Requirements >> README.md
echo. >> README.md
echo - AMD EPYC 9654 (96 cores) or similar >> README.md
echo - 128GB DDR5 ECC RAM >> README.md
echo - NVMe SSDs in RAID 1 >> README.md
echo - Dual 25Gbps Mellanox ConnectX-5 NICs >> README.md
echo - Located in proximity to Solana validator clusters >> README.md
echo. >> README.md
echo ## Installation >> README.md
echo. >> README.md
echo ```bash >> README.md
echo # Clone the repository >> README.md
echo git clone https://github.com/yourusername/solana-hft-bot.git >> README.md
echo cd solana-hft-bot >> README.md
echo. >> README.md
echo # Build in release mode >> README.md
echo cargo build --release >> README.md
echo ``` >> README.md
echo. >> README.md
echo ## Configuration >> README.md
echo. >> README.md
echo TODO: Add configuration instructions >> README.md
echo. >> README.md
echo ## Usage >> README.md
echo. >> README.md
echo ```bash >> README.md
echo ./target/release/solana-hft-bot >> README.md
echo ``` >> README.md
echo. >> README.md
echo ## License >> README.md
echo. >> README.md
echo MIT OR Apache-2.0 >> README.md

REM Create a .gitignore file
echo /target > .gitignore
echo **/*.rs.bk >> .gitignore
echo Cargo.lock >> .gitignore
echo .env >> .gitignore
echo .idea/ >> .gitignore
echo .vscode/ >> .gitignore
echo *.swp >> .gitignore
echo *.swo >> .gitignore
echo *.log >> .gitignore
echo config.*.toml >> .gitignore
echo !config.example.toml >> .gitignore

echo Project structure created successfully!

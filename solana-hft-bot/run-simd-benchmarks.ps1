# PowerShell script to run SIMD benchmarks for the Solana HFT Bot

Write-Host "Building SIMD benchmark..." -ForegroundColor Cyan
cargo build --release --package solana-hft-execution --bin simd_benchmark

Write-Host "Running SIMD benchmarks..." -ForegroundColor Cyan
./target/release/simd_benchmark.exe

Write-Host "Benchmarks complete!" -ForegroundColor Green
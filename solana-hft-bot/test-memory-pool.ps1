# PowerShell script to test the memory pool without Jito feature enabled

# Change to the project directory
Set-Location -Path $PSScriptRoot

# Build and run the memory pool test
Write-Host "Building and running memory pool tests without Jito feature..." -ForegroundColor Green
cargo test --package solana-hft-execution --no-default-features -- queue::tests

# Build and run the standalone memory pool test
Write-Host "Building and running standalone memory pool tests..." -ForegroundColor Green
cargo run --package solana-hft-execution --no-default-features --bin memory_pool_test

Write-Host "All tests completed!" -ForegroundColor Green
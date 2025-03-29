# Test script to verify that the code can be compiled and run with the default features

Write-Host "Testing execution crate with default features..."

# Change to the solana-hft-bot directory
Set-Location -Path $PSScriptRoot

# Build the execution crate with default features
Write-Host "Building execution crate with default features..."
cargo build --package solana-hft-execution

# Check if the build was successful
if ($LASTEXITCODE -eq 0) {
    Write-Host "Build successful!" -ForegroundColor Green
} else {
    Write-Host "Build failed!" -ForegroundColor Red
    exit 1
}

# Run the memory pool test
Write-Host "Running memory pool test..."
cargo run --package solana-hft-execution --bin memory_pool_test

# Check if the test was successful
if ($LASTEXITCODE -eq 0) {
    Write-Host "Memory pool test successful!" -ForegroundColor Green
} else {
    Write-Host "Memory pool test failed!" -ForegroundColor Red
    exit 1
}

# Run the unit tests
Write-Host "Running unit tests with default features..."
cargo test --package solana-hft-execution

# Check if the tests were successful
if ($LASTEXITCODE -eq 0) {
    Write-Host "Unit tests successful!" -ForegroundColor Green
} else {
    Write-Host "Unit tests failed!" -ForegroundColor Red
    exit 1
}

Write-Host "All tests passed!" -ForegroundColor Green
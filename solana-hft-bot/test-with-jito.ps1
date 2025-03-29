# Test script to verify that the code can be compiled and run with the Jito feature enabled

Write-Host "Testing execution crate with Jito feature..."

# Change to the solana-hft-bot directory
Set-Location -Path $PSScriptRoot

# Build the execution crate with the Jito feature
Write-Host "Building execution crate with Jito feature..."
cargo build --package solana-hft-execution --no-default-features --features="jito"

# Check if the build was successful
if ($LASTEXITCODE -eq 0) {
    Write-Host "Build successful!" -ForegroundColor Green
} else {
    Write-Host "Build failed!" -ForegroundColor Red
    exit 1
}

# Run the memory pool test
Write-Host "Running memory pool test..."
cargo run --package solana-hft-execution --no-default-features --features="jito" --bin memory_pool_test

# Check if the test was successful
if ($LASTEXITCODE -eq 0) {
    Write-Host "Memory pool test successful!" -ForegroundColor Green
} else {
    Write-Host "Memory pool test failed!" -ForegroundColor Red
    exit 1
}

# Run the SIMD benchmarks
Write-Host "Running SIMD benchmarks..."
cargo run --package solana-hft-execution --no-default-features --features="jito,simd" --bin simd_benchmark

# Check if the benchmarks were successful
if ($LASTEXITCODE -eq 0) {
    Write-Host "SIMD benchmarks successful!" -ForegroundColor Green
} else {
    Write-Host "SIMD benchmarks failed!" -ForegroundColor Red
    exit 1
}

# Run the unit tests
Write-Host "Running unit tests with Jito feature..."
cargo test --package solana-hft-execution --no-default-features --features="jito"

# Check if the tests were successful
if ($LASTEXITCODE -eq 0) {
    Write-Host "Unit tests successful!" -ForegroundColor Green
} else {
    Write-Host "Unit tests failed!" -ForegroundColor Red
    exit 1
}

# Run the SIMD tests
Write-Host "Running SIMD tests..."
cargo test --package solana-hft-execution --no-default-features --features="jito,simd" -- --nocapture simd::tests

# Check if the SIMD tests were successful
if ($LASTEXITCODE -eq 0) {
    Write-Host "SIMD tests successful!" -ForegroundColor Green
} else {
    Write-Host "SIMD tests failed!" -ForegroundColor Red
    exit 1
}

Write-Host "All tests passed!" -ForegroundColor Green
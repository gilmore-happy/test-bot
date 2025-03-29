@echo off
REM Create project structure
mkdir solana-hft-bot
cd solana-hft-bot

REM Create crates directories
mkdir crates
mkdir crates\core crates\network crates\execution crates\arbitrage crates\screening crates\risk crates\rpc crates\cli

REM Create src directories
mkdir crates\core\src
mkdir crates\network\src
mkdir crates\execution\src
mkdir crates\screening\src
mkdir crates\arbitrage\src
mkdir crates\risk\src
mkdir crates\rpc\src
mkdir crates\cli\src

echo Basic project structure created successfully!

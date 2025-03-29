@echo off
REM Backup script for Solana HFT Bot Docker environment

echo Solana HFT Bot - Backup Script
echo ==============================

REM Set variables
set BACKUP_DIR=backups
set TIMESTAMP=%date:~10,4%-%date:~4,2%-%date:~7,2%_%time:~0,2%-%time:~3,2%-%time:~6,2%
set TIMESTAMP=%TIMESTAMP: =0%
set BACKUP_FILE=%BACKUP_DIR%\solana-hft-backup-%TIMESTAMP%.tar

REM Create backup directory if it doesn't exist
if not exist %BACKUP_DIR% (
    echo Creating backup directory...
    mkdir %BACKUP_DIR%
)

echo.
echo Creating backup at %BACKUP_FILE%...
echo.

REM Backup Docker volumes
echo Backing up Docker volumes...
docker run --rm -v solana-hft_config-volume:/config -v solana-hft_keys-volume:/keys -v solana-hft_db-data:/db -v %cd%\%BACKUP_DIR%:/backup alpine tar -cf /backup/volumes-%TIMESTAMP%.tar /config /keys /db
if %ERRORLEVEL% neq 0 (
    echo Error backing up Docker volumes!
    exit /b 1
)

REM Backup configuration files
echo Backing up configuration files...
tar -cf %BACKUP_FILE% solana-hft-bot\config.json solana-hft-bot\logging-config.json solana-hft-bot\metrics-config.json prometheus.yml docker-compose.yml
if %ERRORLEVEL% neq 0 (
    echo Error backing up configuration files!
    exit /b 1
)

REM Export database
echo Exporting database...
docker exec solana-hft-db pg_dump -U postgres -d hft > %BACKUP_DIR%\db-backup-%TIMESTAMP%.sql
if %ERRORLEVEL% neq 0 (
    echo Error exporting database!
    exit /b 1
)

echo.
echo Backup completed successfully!
echo Backup files:
echo - %BACKUP_FILE%
echo - %BACKUP_DIR%\volumes-%TIMESTAMP%.tar
echo - %BACKUP_DIR%\db-backup-%TIMESTAMP%.sql
echo.

exit /b 0
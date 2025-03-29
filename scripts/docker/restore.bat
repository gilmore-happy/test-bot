@echo off
REM Restore script for Solana HFT Bot Docker environment

echo Solana HFT Bot - Restore Script
echo ==============================

REM Check if backup file is provided
if "%~1"=="" (
    echo Error: No backup file specified!
    echo Usage: restore.bat [backup_file] [volumes_backup] [db_backup]
    exit /b 1
)

set BACKUP_FILE=%~1
set VOLUMES_BACKUP=%~2
set DB_BACKUP=%~3

REM Validate backup files
if not exist %BACKUP_FILE% (
    echo Error: Backup file %BACKUP_FILE% not found!
    exit /b 1
)

if not "%VOLUMES_BACKUP%"=="" (
    if not exist %VOLUMES_BACKUP% (
        echo Error: Volumes backup file %VOLUMES_BACKUP% not found!
        exit /b 1
    )
)

if not "%DB_BACKUP%"=="" (
    if not exist %DB_BACKUP% (
        echo Error: Database backup file %DB_BACKUP% not found!
        exit /b 1
    )
)

echo.
echo Restoring from backup %BACKUP_FILE%...
echo.

REM Stop running containers
echo Stopping running containers...
docker-compose down
if %ERRORLEVEL% neq 0 (
    echo Warning: Error stopping containers, continuing anyway...
)

REM Restore configuration files
echo Restoring configuration files...
tar -xf %BACKUP_FILE%
if %ERRORLEVEL% neq 0 (
    echo Error restoring configuration files!
    exit /b 1
)

REM Restore Docker volumes if provided
if not "%VOLUMES_BACKUP%"=="" (
    echo Restoring Docker volumes...
    docker run --rm -v solana-hft_config-volume:/config -v solana-hft_keys-volume:/keys -v solana-hft_db-data:/db -v %cd%:/backup alpine sh -c "rm -rf /config/* /keys/* /db/* && tar -xf /backup/%VOLUMES_BACKUP% -C /"
    if %ERRORLEVEL% neq 0 (
        echo Error restoring Docker volumes!
        exit /b 1
    )
)

REM Start containers
echo Starting containers...
docker-compose up -d
if %ERRORLEVEL% neq 0 (
    echo Error starting containers!
    exit /b 1
)

REM Restore database if provided
if not "%DB_BACKUP%"=="" (
    echo Waiting for database to be ready...
    timeout /t 10 /nobreak > nul
    
    echo Restoring database...
    type %DB_BACKUP% | docker exec -i solana-hft-db psql -U postgres -d hft
    if %ERRORLEVEL% neq 0 (
        echo Error restoring database!
        exit /b 1
    )
)

echo.
echo Restore completed successfully!
echo.

exit /b 0
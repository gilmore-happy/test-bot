@echo off
REM Security scanning script for Solana HFT Bot Docker environment

echo Solana HFT Bot - Security Scanning Script
echo =======================================

REM Set variables
set SCAN_DIR=security-scans
set TIMESTAMP=%date:~10,4%-%date:~4,2%-%date:~7,2%_%time:~0,2%-%time:~3,2%-%time:~6,2%
set TIMESTAMP=%TIMESTAMP: =0%

REM Create scan directory if it doesn't exist
if not exist %SCAN_DIR% (
    mkdir %SCAN_DIR%
)

echo.
echo Starting security scans at %TIMESTAMP%...
echo.

REM Check Docker daemon configuration
echo Checking Docker daemon configuration...
docker info > %SCAN_DIR%\docker-info-%TIMESTAMP%.txt
echo Results saved to %SCAN_DIR%\docker-info-%TIMESTAMP%.txt

REM Check container security settings
echo Checking container security settings...
docker-compose config > %SCAN_DIR%\docker-compose-config-%TIMESTAMP%.txt
echo Results saved to %SCAN_DIR%\docker-compose-config-%TIMESTAMP%.txt

REM Check running containers
echo Checking running containers...
docker ps --format "table {{.ID}}\t{{.Image}}\t{{.Command}}\t{{.CreatedAt}}\t{{.Status}}\t{{.Ports}}\t{{.Names}}" > %SCAN_DIR%\running-containers-%TIMESTAMP%.txt
echo Results saved to %SCAN_DIR%\running-containers-%TIMESTAMP%.txt

REM Check container resource limits
echo Checking container resource limits...
docker-compose ps -q | findstr /r /c:"." > nul
if %ERRORLEVEL% equ 0 (
    for /f %%i in ('docker-compose ps -q') do (
        docker inspect --format="{{.Name}}: {{.HostConfig.Resources}}" %%i >> %SCAN_DIR%\container-limits-%TIMESTAMP%.txt
    )
    echo Results saved to %SCAN_DIR%\container-limits-%TIMESTAMP%.txt
) else (
    echo No running containers found.
)

echo.
echo Security scan completed. All results saved to %SCAN_DIR% directory.
echo.

exit /b 0
@echo off
REM Monitoring script for Solana HFT Bot Docker environment

echo Solana HFT Bot - Monitoring Script
echo =================================

REM Set variables
set LOG_DIR=logs
set TIMESTAMP=%date:~10,4%-%date:~4,2%-%date:~7,2%_%time:~0,2%-%time:~3,2%-%time:~6,2%
set TIMESTAMP=%TIMESTAMP: =0%

REM Create log directory if it doesn't exist
if not exist %LOG_DIR% (
    mkdir %LOG_DIR%
)

REM Check container status
echo Checking container status...
docker-compose ps > %LOG_DIR%\container-status-%TIMESTAMP%.log
echo Container status saved to %LOG_DIR%\container-status-%TIMESTAMP%.log

REM Check container resource usage
echo Checking container resource usage...
docker stats --no-stream > %LOG_DIR%\container-stats-%TIMESTAMP%.log
echo Container stats saved to %LOG_DIR%\container-stats-%TIMESTAMP%.log

REM Check container logs
echo Checking container logs...
docker-compose logs --tail=100 > %LOG_DIR%\container-logs-%TIMESTAMP%.log
echo Container logs saved to %LOG_DIR%\container-logs-%TIMESTAMP%.log

REM Check disk space
echo Checking disk space...
wmic logicaldisk get deviceid,freespace,size > %LOG_DIR%\disk-space-%TIMESTAMP%.log
echo Disk space info saved to %LOG_DIR%\disk-space-%TIMESTAMP%.log

REM Check Docker volume usage
echo Checking Docker volume usage...
docker system df -v > %LOG_DIR%\docker-volumes-%TIMESTAMP%.log
echo Docker volume usage saved to %LOG_DIR%\docker-volumes-%TIMESTAMP%.log

echo.
echo Monitoring completed. All logs saved to %LOG_DIR% directory.
echo.

exit /b 0
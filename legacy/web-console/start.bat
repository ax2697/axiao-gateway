@echo off
chcp 65001 >nul
cd /d "%~dp0"
echo [axiao-shell] starting...
start "" http://127.0.0.1:7870
node server.mjs
pause

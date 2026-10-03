@echo off
rem 文台 App 一键构建入口（Windows）
setlocal
set "DIR=%~dp0"

python "%DIR%scripts\build.py" %*
if errorlevel 1 (
  echo.
  echo 构建未完成，请根据上方提示补齐工具链后重试。
  exit /b %errorlevel%
)

endlocal

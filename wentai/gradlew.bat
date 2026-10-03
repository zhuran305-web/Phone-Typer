@rem Gradle wrapper startup script for Windows
@if "%DEBUG%"=="" @echo off
@rem Set local scope for the variables with windows NT shell
if "%OS%"=="Windows_NT" setlocal

set DIRNAME=%~dp0
if "%DIRNAME%"=="" set DIRNAME=.
@rem This is normally unused
set APP_BASE_NAME=%~n0
set APP_HOME=%DIRNAME%

set CLASSNAME=org.gradle.wrapper.GradleWrapperMain
set CLASSPATH=%APP_HOME%\gradle\wrapper\gradle-wrapper.jar

if not exist "%CLASSPATH%" (
  echo.
  echo [ERROR] 未找到 gradle-wrapper.jar: %CLASSPATH%
  echo 请先安装 Gradle 并执行: gradle wrapper --gradle-version 8.7
  echo 或使用 wentai\build.bat 自动引导构建环境。
  exit /b 1
)

set DEFAULT_JVM_OPTS="-Xmx64m" "-Xms64m"

if defined JAVA_HOME (
  set JAVA_EXE=%JAVA_HOME%\bin\java.exe
) else (
  set JAVA_EXE=java.exe
)

"%JAVA_EXE%" %DEFAULT_JVM_OPTS% %JAVA_OPTS% %GRADLE_OPTS% -classpath "%CLASSPATH%" %CLASSNAME% %*

endlocal

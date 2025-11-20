@echo off
ECHO Building Prometheus ...
RMDIR /s /q build
MKDIR build
glue.exe ./srlua.exe prometheus-main.lua build/prometheus.exe
robocopy ./src ./build/lua /E>nul

robocopy . ./build lua51.dll>nul

ECHO Done!
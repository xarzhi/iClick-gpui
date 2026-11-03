@echo off
rem iClick-gpui 打包入口：pack 1.5.0 / pack 1.5.0 -NoZip
rem 实际逻辑在 pack.ps1 里，这里只是让你在任何终端里都能直接敲 pack。
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0pack.ps1" %*

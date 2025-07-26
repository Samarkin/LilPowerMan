@echo off
cargo build %*

set DIR=debug
for %%a in (%*) do (
    if "%%a"=="-r" (
        set DIR=release
        goto :copy
    )
    if "%%a"=="--release" (
        set DIR=release
        goto :copy
    )
)

:copy
xcopy ryzenadj\* target\%DIR% /i /y

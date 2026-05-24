cargo build --release --target x86_64-pc-windows-msvc
if %errorlevel% equ 0 (
    echo [+] Agent DLL built: target\x86_64-pc-windows-msvc\release\c2_agent.dll
) else (
    echo [!] Build failed
)

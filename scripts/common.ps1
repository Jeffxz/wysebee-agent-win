# What the scripts share. Dot-sourced (. "$PSScriptRoot\common.ps1"), never run. Windows only.
#
# ASCII only, comments included: Windows PowerShell 5.1 reads a BOM-less UTF-8 script as
# ANSI, where a multi-byte character can end a string early (Wysebee Desktop's rule).

# Run a program with its arguments; stop the script when it fails.
#
# A plain function on purpose: an advanced one would take `-p` for a parameter of its own.
# PowerShell swallows a bare -- given to a function: write it '--'.
#
# ErrorActionPreference drops to Continue for the call: cargo writes its progress to
# stderr, and Windows PowerShell 5.1 turns a native command's stderr into an error while
# the preference is Stop, so a good build would report itself failed on its first
# "Compiling" line. $LASTEXITCODE is what decides.
function Invoke-Native {
    $Program, $Arguments = $args
    $previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        & $Program @Arguments
    } finally {
        $ErrorActionPreference = $previous
    }
    if ($LASTEXITCODE -ne 0) {
        throw "$Program $($Arguments -join ' ') failed (exit code $LASTEXITCODE)."
    }
}

# Stop unless this is Windows.
function Assert-Windows([string]$What) {
    if ($env:OS -ne 'Windows_NT') {
        [Console]::Error.WriteLine("$What runs on Windows only.")
        exit 1
    }
}

# cargo on the PATH, or in rustup's default place; else how to get it.
function Assert-Cargo {
    if (Get-Command cargo -ErrorAction SilentlyContinue) { return }
    $CargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
    if (Test-Path (Join-Path $CargoBin 'cargo.exe')) {
        $env:PATH = "$CargoBin;$env:PATH"
        return
    }
    [Console]::Error.WriteLine('cargo not found. Install Rust (https://rustup.rs, the default x86_64-pc-windows-msvc toolchain) and the Visual Studio C++ Build Tools ("Desktop development with C++").')
    exit 1
}

# The app's version: [workspace.package] in the workspace Cargo.toml, where it lives and
# only there (the [workspace.dependencies] lines below it carry versions too).
function Get-WorkspaceVersion([string]$RepoRoot) {
    $section = ''
    foreach ($line in Get-Content (Join-Path $RepoRoot 'Cargo.toml')) {
        if ($line -match '^\s*\[([^\]]+)\]\s*$') {
            $section = $Matches[1].Trim()
            continue
        }
        if ($section -eq 'workspace.package' -and $line -match '^\s*version\s*=\s*"([^"]+)"') {
            return $Matches[1]
        }
    }
    throw "No version in Cargo.toml's [workspace.package]."
}

# A PE file's subsystem: 2 is a Windows GUI program, 3 a console one (which opens a
# console window beside the app).
function Get-PeSubsystem([string]$Path) {
    $bytes = [System.IO.File]::ReadAllBytes($Path)
    $pe = [BitConverter]::ToInt32($bytes, 0x3C)
    # The optional header starts 24 bytes after "PE\0\0"; its Subsystem field is at 68.
    return [BitConverter]::ToUInt16($bytes, $pe + 24 + 68)
}

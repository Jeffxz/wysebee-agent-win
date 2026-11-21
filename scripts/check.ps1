<#
.SYNOPSIS
    Everything must pass, every time: clippy with no warning, every test, and 100% of
    agent-core's lines covered. Run it before every commit.

.DESCRIPTION
    scripts\check.ps1

    Left out of the coverage rule, and nothing else:
      - crates\agent-core\src\secrets\credentials.rs, the real Credential Manager, which no
        test may touch;
      - crates\agent-core\src\links.rs, the tests' own helper for making links.
    (tests\, with the tests' stand-in program in tests\support\, is never measured.) Cover new
    code with a real test, never with an exclusion.

    The tests make links: a folder link is a junction (no privilege needed), a file link a
    symbolic link, which Windows lets a person make only with Developer Mode on (Settings >
    System > For developers) or as an administrator. Without it the tests that need one
    fail, so this script asks for it first.

    agent-app (the Tauri shell) is built and linted here but not measured: it is the window
    and the real Options, and holds no logic the core's tests do not already cover.
#>
param()

$ErrorActionPreference = 'Stop'

$RepoRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'common.ps1')
Assert-Windows 'check.ps1'
Assert-Cargo

cargo llvm-cov --version *> $null
if ($LASTEXITCODE -ne 0) {
    throw 'cargo-llvm-cov is needed: cargo install cargo-llvm-cov (and rustup component add llvm-tools-preview).'
}

# A file link, made and removed: can the tests make one here?
$Probe = Join-Path ([System.IO.Path]::GetTempPath()) "wysebee-link-probe-$PID"
try {
    New-Item -ItemType SymbolicLink -Path $Probe -Target $PSCommandPath -ErrorAction Stop | Out-Null
    Remove-Item -Force $Probe
} catch {
    throw 'The tests need to make a file link: turn on Developer Mode (Settings > System > For developers), or run this as an administrator.'
}

Push-Location $RepoRoot
try {
    Write-Host '== clippy'
    Invoke-Native cargo clippy --workspace --all-targets '--' -D warnings

    Write-Host '== tests'
    Invoke-Native cargo test --workspace

    Write-Host "== coverage (agent-core, 100% of lines)"
    # --fail-uncovered-lines, not --fail-under-lines: the TOTAL row counts each test binary's
    # own copy of a function, so it stays under 100% even when every line is run.
    Invoke-Native cargo llvm-cov -p agent-core --show-missing-lines `
        --ignore-filename-regex 'agent-core[\\/]src[\\/](secrets[\\/]credentials|links)\.rs$' `
        --fail-uncovered-lines 0
} finally {
    Pop-Location
}

Write-Host 'All green.'

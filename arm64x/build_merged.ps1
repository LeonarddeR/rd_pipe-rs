# Build a merged ARM64X DLL from the aarch64-pc-windows-msvc and
# arm64ec-pc-windows-msvc staticlibs of rd_pipe.
#
#   -Arm64Lib   : path to target\aarch64-pc-windows-msvc\release\rd_pipe.lib
#   -Arm64ecLib : path to target\arm64ec-pc-windows-msvc\release\rd_pipe.lib
#   -OutDir     : directory to place the merged rd_pipe.dll
#
# Final link uses /MACHINE:ARM64X via rust-lld from the active rustc
# toolchain. Explicit Windows SDK import libs are passed because Rust
# staticlibs do not embed them; they reference SDK symbols via
# raw_dylib stubs that the ARM64X linker cannot resolve without proper
# import libraries.

[CmdletBinding()]
param(
    [Parameter(Mandatory)] [string] $Arm64Lib,
    [Parameter(Mandatory)] [string] $Arm64ecLib,
    [Parameter(Mandatory)] [string] $OutDir
)

$ErrorActionPreference = 'Stop'

# Resolve rust-lld bundled with the active rustc toolchain.
$sysroot = (& rustc --print sysroot).Trim()
$hostTriple = (& rustc -vV | Select-String '^host:').ToString().Split(' ', 2)[1].Trim()
$linkExe = Join-Path $sysroot "lib\rustlib\$hostTriple\bin\gcc-ld\lld-link.exe"
if (-not (Test-Path -LiteralPath $linkExe)) {
    throw "rust-lld not found at $linkExe"
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
# Same DEF used for ARM64 native and ARM64EC views; both export the same
# COM entry points.
$def = Join-Path $PSScriptRoot 'rd_pipe.def'

# Windows SDK import libs needed by both Rust halves (kernel32, advapi32,
# bcrypt, ntdll, oleaut32, userenv, ws2_32) plus the C runtime stubs the
# ARM64EC half pulls in (msvcrt, vcruntime).
$sdkLibs = @(
    'kernel32.lib',
    'advapi32.lib',
    'bcrypt.lib',
    'ntdll.lib',
    'ole32.lib',
    'oleaut32.lib',
    'userenv.lib',
    'ws2_32.lib',
    'synchronization.lib',
    'msvcrt.lib',
    'ucrt.lib',
    'vcruntime.lib',
    'softintrin.lib'
)

$outDll = Join-Path $OutDir 'rd_pipe.dll'

Write-Host "==> Linking ARM64X merged DLL with $linkExe"
# /entry:DllMain ensures the loader calls our Rust DllMain rather than the
# msvcrt stub (msvcrt provides a no-op _DllMainCRTStartup; we want our own).
# /force:multiple resolves the resulting duplicate-symbol diagnostic in
# favor of the input listed first (the Rust staticlibs).
# /defArm64Native + /def supply the export tables for the ARM64 native and
# ARM64EC views respectively; the same DEF is used for both since the
# Rust crate exports the same COM entry points on both ABIs.
& $linkExe `
    /dll /machine:arm64x /nologo `
    /noimplib `
    /entry:DllMain `
    /force:multiple `
    "/defArm64Native:$def" `
    "/def:$def" `
    "/out:$outDll" `
    $Arm64Lib $Arm64ecLib `
    @sdkLibs
if ($LASTEXITCODE -ne 0) { throw "rust-lld failed" }

Write-Host "==> Merged ARM64X DLL built: $outDll"
Get-Item $outDll | Format-List FullName, Length, LastWriteTime

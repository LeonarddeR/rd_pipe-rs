# Build a merged ARM64X DLL from the aarch64-pc-windows-msvc and
# arm64ec-pc-windows-msvc staticlibs of rd_pipe.
#
#   -Arm64Lib   : path to target\aarch64-pc-windows-msvc\release\rd_pipe.lib
#   -Arm64ecLib : path to target\arm64ec-pc-windows-msvc\release\rd_pipe.lib
#   -OutDir     : directory to place the merged rd_pipe.dll
#   -Linker     : 'lld-link' (default) or 'link'. lld-link ships with the
#                 MSVC Clang component; link.exe ships with MSVC.
#
# Final link uses /MACHINE:ARM64X. Explicit Windows SDK import libs are
# passed because Rust staticlibs do not embed them; they reference SDK
# symbols via raw_dylib stubs that the ARM64X linker cannot resolve
# without proper import libraries.
#
# NOTE: tracking https://github.com/rust-lang/rust/issues/145154 -- on
# rustc with LLVM <22.1.0, the resulting ARM64X DLL still crashes inside
# x64/ARM64EC processes (works fine inside ARM64 processes). The output
# of this script is therefore only fully usable once a rustc with
# LLVM >=22.1.0 ships.

[CmdletBinding()]
param(
    [Parameter(Mandatory)] [string] $Arm64Lib,
    [Parameter(Mandatory)] [string] $Arm64ecLib,
    [Parameter(Mandatory)] [string] $OutDir,
    [ValidateSet('lld-link', 'link')] [string] $Linker = 'lld-link'
)

$ErrorActionPreference = 'Stop'

function Resolve-Tool {
    param([string[]]$Names)
    foreach ($n in $Names) {
        $cmd = Get-Command $n -ErrorAction SilentlyContinue
        if ($cmd) { return $cmd.Source }
    }
    throw "could not locate any of: $($Names -join ', ')"
}

# Final link can be either MSVC link.exe or LLVM lld-link. Both
# synthesize the ARM64X dynamic value table when given /machine:arm64x
# and a matched pair of /defArm64Native + /def export descriptions.
$linkExe = Resolve-Tool -Names @($Linker)

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

Write-Host "==> Linking ARM64X merged DLL with $Linker"
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
if ($LASTEXITCODE -ne 0) { throw "$Linker failed" }

Write-Host "==> Merged ARM64X DLL built: $outDll"
Get-Item $outDll | Format-List FullName, Length, LastWriteTime

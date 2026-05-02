# Build a merged ARM64X DLL from the aarch64-pc-windows-msvc and
# arm64ec-pc-windows-msvc staticlibs of rd_pipe.
#
#   -Arm64Lib   : path to target\aarch64-pc-windows-msvc\release\rd_pipe.lib
#   -Arm64ecLib : path to target\arm64ec-pc-windows-msvc\release\rd_pipe.lib
#   -Arm64Dll   : path to target\aarch64-pc-windows-msvc\release\rd_pipe.dll
#   -Arm64ecDll : path to target\arm64ec-pc-windows-msvc\release\rd_pipe.dll
#   -OutDir     : directory to place the merged rd_pipe.dll
#
# Final link uses /MACHINE:ARM64X via rust-lld from the active rustc
# toolchain. Explicit Windows SDK import libs are passed because Rust
# staticlibs do not embed them; they reference SDK symbols via
# raw_dylib stubs that the ARM64X linker cannot resolve without proper
# import libraries.
#
# DEF files are generated on the fly from the per-arch DLLs via
# llvm-readobj --coff-exports so the merged binary's export table is the
# exact union of the per-arch tables (DllMain, DllGetClassObject,
# DllInstall, plus anything Rust adds in the future). Avoids drift
# between a hand-maintained DEF and what rustc actually exports.

[CmdletBinding()]
param(
    [Parameter(Mandatory)] [string] $Arm64Lib,
    [Parameter(Mandatory)] [string] $Arm64ecLib,
    [Parameter(Mandatory)] [string] $Arm64Dll,
    [Parameter(Mandatory)] [string] $Arm64ecDll,
    [Parameter(Mandatory)] [string] $OutDir
)

$ErrorActionPreference = 'Stop'

# Resolve rust-lld + llvm-readobj bundled with the active rustc toolchain.
# llvm-readobj requires the `llvm-tools` rustup component.
$sysroot = (& rustc --print sysroot).Trim()
$hostTriple = (& rustc -vV | Select-String '^host:').ToString().Split(' ', 2)[1].Trim()
$rustlibBin = Join-Path $sysroot "lib\rustlib\$hostTriple\bin"
$linkExe = Join-Path $rustlibBin 'gcc-ld\lld-link.exe'
$readobjExe = Join-Path $rustlibBin 'llvm-readobj.exe'
if (-not (Test-Path -LiteralPath $linkExe)) {
    throw "rust-lld not found at $linkExe"
}
if (-not (Test-Path -LiteralPath $readobjExe)) {
    throw "llvm-readobj not found at $readobjExe (install rustup component 'llvm-tools')"
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

# Generate a DEF file from a per-arch DLL via `llvm-readobj --coff-exports`.
# Output format:
#   Export {
#     Ordinal: <n>
#     Name: <symbol>
#     RVA: 0x<hex>
#   }
# We extract every `Name:` line. Order doesn't matter for /def.
function New-DefFromDll {
    param([string]$Dll, [string]$DefPath)
    $names = & $readobjExe --coff-exports $Dll |
        Select-String '^\s*Name: (\S+)$' |
        ForEach-Object { $_.Matches[0].Groups[1].Value }
    if (-not $names) {
        throw "No exports found in $Dll via $readobjExe"
    }
    $lines = @('EXPORTS') + ($names | ForEach-Object { "    $_" })
    Set-Content -Path $DefPath -Value $lines -Encoding ASCII
    Write-Host "==> Generated $DefPath ($($names.Count) exports): $($names -join ', ')"
}

$defArm64 = Join-Path $OutDir 'rd_pipe.arm64.def'
$defArm64ec = Join-Path $OutDir 'rd_pipe.arm64ec.def'
New-DefFromDll -Dll $Arm64Dll   -DefPath $defArm64
New-DefFromDll -Dll $Arm64ecDll -DefPath $defArm64ec

# Minimum Windows SDK / CRT import libs needed for the link to succeed.
# Determined empirically by drop-one probing against rust-lld 22.1.2:
# - kernel32.lib  : __chkstk, _tls_index/_tls_used, baseline Win32
# - msvcrt.lib    : _CxxThrowException, __CxxFrameHandler3 (ARM64EC)
# - ucrt.lib      : memcpy/memset/memmove/memcmp, wcslen, etc.
# - vcruntime.lib : softintrin / icall helpers (ARM64EC)
# All other Win32 imports (advapi32, bcrypt, ntdll, ole32, oleaut32,
# userenv, ws2_32, synchronization) are pulled in transitively via the
# Rust staticlibs' raw_dylib stubs.
$sdkLibs = @(
    'kernel32.lib',
    'msvcrt.lib',
    'ucrt.lib',
    'vcruntime.lib'
)

$outDll = Join-Path $OutDir 'rd_pipe.dll'

Write-Host "==> Linking ARM64X merged DLL with $linkExe"
# /entry:DllMain ensures the loader calls our Rust DllMain rather than the
# msvcrt stub (msvcrt provides a no-op _DllMainCRTStartup; we want our own).
# /force:multiple resolves the resulting duplicate-symbol diagnostic in
# favor of the input listed first (the Rust staticlibs).
# /defArm64Native + /def supply the export tables for the ARM64 native and
# ARM64EC views respectively. Each DEF is generated from the matching
# per-arch DLL so the merged binary exposes the exact union of exports
# rustc emitted for each arch.
& $linkExe `
    /dll /machine:arm64x /nologo `
    /noimplib `
    /entry:DllMain `
    /force:multiple `
    "/defArm64Native:$defArm64" `
    "/def:$defArm64ec" `
    "/out:$outDll" `
    $Arm64Lib $Arm64ecLib `
    @sdkLibs
if ($LASTEXITCODE -ne 0) { throw "rust-lld failed" }

Write-Host "==> Merged ARM64X DLL built: $outDll"
Get-Item $outDll | Format-List FullName, Length, LastWriteTime

# Build a merged ARM64X DLL from the aarch64-pc-windows-msvc and
# arm64ec-pc-windows-msvc staticlibs of rd_pipe.
#
#   -Arm64Lib   : path to target\aarch64-pc-windows-msvc\release\rd_pipe.lib
#   -Arm64ecLib : path to target\arm64ec-pc-windows-msvc\release\rd_pipe.lib
#   -Arm64Dll   : path to target\aarch64-pc-windows-msvc\release\rd_pipe.dll
#   -Arm64ecDll : path to target\arm64ec-pc-windows-msvc\release\rd_pipe.dll
#   -OutDir     : directory to place the merged rd_pipe.dll
#
# Final link uses /MACHINE:ARM64X via MSVC link.exe (discovered from the
# active VS installation via vswhere), which handles the ARM64X TLS directory
# correctly. lld-link is NOT used because it corrupts the EC view's TLS
# (_tls_used/_tls_index/TLS-callback directory) when merging two full Rust
# staticlibs.
#
# Import libs are passed explicitly by full path for both the ARM64 (native)
# and x64 (EC) views because an /machine:arm64x link needs import libraries
# for both architectures. Bare lib names resolved from ambient LIB only cover
# one architecture.
#
# DEF files are generated on the fly from the per-arch DLLs via
# `dumpbin /exports` (same VS install, no extra components needed) so the
# merged binary's export table is the exact union of the per-arch tables
# (DllMain, DllGetClassObject, DllInstall, plus anything Rust adds in the
# future). Avoids drift between a hand-maintained DEF and what rustc exports.

[CmdletBinding()]
param(
    [Parameter(Mandatory)] [string] $Arm64Lib,
    [Parameter(Mandatory)] [string] $Arm64ecLib,
    [Parameter(Mandatory)] [string] $Arm64Dll,
    [Parameter(Mandatory)] [string] $Arm64ecDll,
    [Parameter(Mandatory)] [string] $OutDir
)

$ErrorActionPreference = 'Stop'

# Resolve MSVC link.exe and dumpbin.exe. Both come from the same VS VC tools
# install — no extra rustup components required.
# Prefers ARM64-hosted binaries (run natively); falls back to x64-hosted.
# Uses VCToolsInstallDir env (set by vcvars) or discovers via vswhere.
function Get-MsvcTool {
    param([string]$Name)
    $vcRoot = $null
    if ($env:VCToolsInstallDir) {
        $vcRoot = $env:VCToolsInstallDir.TrimEnd('\/')
    } else {
        $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
        if (-not (Test-Path -LiteralPath $vswhere)) { throw "vswhere.exe not found at $vswhere" }
        $vsRoot = (& $vswhere -latest -prerelease -products * `
            -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
            -property installationPath | Select-Object -First 1)
        if (-not $vsRoot) { $vsRoot = (& $vswhere -latest -prerelease -products * -property installationPath | Select-Object -First 1) }
        if (-not $vsRoot) { throw "No Visual Studio with VC tools found via vswhere" }
        $verFile = Join-Path $vsRoot 'VC\Auxiliary\Build\Microsoft.VCToolsVersion.default.txt'
        if (-not (Test-Path -LiteralPath $verFile)) { throw "VC tools version file not found at $verFile" }
        $ver = (Get-Content -LiteralPath $verFile -Raw).Trim()
        $vcRoot = Join-Path $vsRoot "VC\Tools\MSVC\$ver"
    }
    foreach ($hostDir in 'Hostarm64\arm64', 'HostARM64\arm64', 'Hostx64\x64') {
        $p = Join-Path $vcRoot "bin\$hostDir\$Name"
        if (Test-Path -LiteralPath $p) { return $p }
    }
    throw "MSVC $Name not found under $vcRoot\bin"
}

function Get-MsvcLibRoot {
    # Derive lib root from VCToolsInstallDir or the same vswhere path Get-MsvcTool uses.
    if ($env:VCToolsInstallDir) {
        $r = Join-Path $env:VCToolsInstallDir.TrimEnd('\/') 'lib'
        if (Test-Path -LiteralPath $r) { return (Resolve-Path -LiteralPath $r).Path }
    }
    # Get-MsvcTool already validated the VS install; re-derive the lib path from it.
    $linkExePath = Get-MsvcTool 'link.exe'   # e.g. ...\bin\Hostarm64\arm64\link.exe
    # Strip \arm64, \Hostarm64, \bin to reach the MSVC version root
    $vcVersionRoot = Split-Path (Split-Path (Split-Path (Split-Path $linkExePath)))
    $root = Join-Path $vcVersionRoot 'lib'
    if (-not (Test-Path -LiteralPath $root)) { throw "MSVC lib root not found at $root" }
    return $root
}

function Get-SdkLibRoot {
    $kitsRoot = $null
    foreach ($hive in 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Microsoft SDKs\Windows\v10.0',
                      'HKLM:\SOFTWARE\Microsoft\Microsoft SDKs\Windows\v10.0') {
        $p = (Get-ItemProperty -Path $hive -Name 'KitsRoot10' -ErrorAction SilentlyContinue).KitsRoot10
        if ($p) { $kitsRoot = $p; break }
    }
    if (-not $kitsRoot) { $kitsRoot = 'C:\Program Files (x86)\Windows Kits\10' }
    $libBase = Join-Path $kitsRoot 'Lib'
    if (-not (Test-Path -LiteralPath $libBase)) { throw "Windows SDK Lib base not found at $libBase" }
    # Pick the highest-versioned SDK that ships both arm64 and x64 ucrt/um libs.
    $sdk = Get-ChildItem -LiteralPath $libBase -Directory |
        Where-Object { (Test-Path (Join-Path $_.FullName 'um\arm64')) -and
                       (Test-Path (Join-Path $_.FullName 'um\x64')) } |
        Sort-Object { [version]$_.Name } -Descending |
        Select-Object -First 1
    if (-not $sdk) {
        throw "No Windows SDK with arm64 + x64 um libs found under $libBase."
    }
    return $sdk.FullName
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

$linkExe     = Get-MsvcTool 'link.exe'
$dumpbinExe  = Get-MsvcTool 'dumpbin.exe'
$msvcLibRoot = Get-MsvcLibRoot
$sdkLibRoot  = Get-SdkLibRoot

Write-Host "==> MSVC link.exe:    $linkExe"
Write-Host "==> MSVC dumpbin.exe: $dumpbinExe"
Write-Host "==> MSVC lib root:    $msvcLibRoot"
Write-Host "==> Windows SDK root: $sdkLibRoot"

# Generate a DEF file from a per-arch DLL via `dumpbin /exports`.
# Output format includes lines like:
#   <ordinal>  <hint>  <RVA>  <name>
# We match the name in the 4th field of export lines.
function New-DefFromDll {
    param([string]$Dll, [string]$DefPath)
    $names = & $dumpbinExe /exports $Dll 2>&1 |
        Select-String '^\s+\d+\s+[0-9A-Fa-f]+\s+[0-9A-Fa-f]+\s+(\w+)' |
        ForEach-Object { $_.Matches[0].Groups[1].Value }
    if (-not $names) {
        throw "No exports found in $Dll via $dumpbinExe"
    }
    $lines = @('EXPORTS') + ($names | ForEach-Object { "    $_" })
    Set-Content -Path $DefPath -Value $lines -Encoding ASCII
    Write-Host "==> Generated $DefPath ($($names.Count) exports): $($names -join ', ')"
}

$defArm64   = Join-Path $OutDir 'rd_pipe.arm64.def'
$defArm64ec = Join-Path $OutDir 'rd_pipe.arm64ec.def'
New-DefFromDll -Dll $Arm64Dll   -DefPath $defArm64
New-DefFromDll -Dll $Arm64ecDll -DefPath $defArm64ec

# Import libs needed for the /machine:arm64x link.
# Mirror exactly what rustc passes for each architecture (from `cargo rustc -- --print link-args`):
#   ARM64 (native view): kernel32 ntdll userenv ws2_32 dbghelp  + /defaultlib:msvcrt
#   ARM64EC (EC view):   kernel32 ntdll userenv ws2_32 dbghelp  + /defaultlib:msvcrt + softintrin
#
# An ARM64X link has TWO symbol namespaces; we need CRT + SDK libs for BOTH:
#   - MSVC arm64\ for both views (msvcrt, vcruntime, ucrt-style helpers)
#   - SDK ucrt\arm64  for the native view
#   - SDK ucrt\arm64ec (or x64 fallback) for the EC view
#   - SDK um\arm64    for the native view
#   - SDK um\arm64ec  (or x64 fallback) for the EC view
#   - softintrin (EC/arm64ec only)
#
# Note: __icall_helper_arm64ec lives in arm64\msvcrt.lib (not x64 or arm64ec).
# Note: SDK 10.0.28000 does not have a um\arm64ec dir; fall back to um\x64.
function Resolve-LibForView {
    param([string]$Name, [string]$Arch)  # Arch = 'arm64' | 'arm64ec'
    # CRT libs from MSVC: always use arm64 dir (same helpers serve both views)
    if ($Name -match '^(msvcrt|vcruntime)\.lib$') {
        $p = Join-Path $msvcLibRoot "arm64\$Name"
        if (-not (Test-Path -LiteralPath $p)) { throw "MSVC lib not found: $p" }
        return $p
    }
    # SDK ucrt
    if ($Name -match '^ucrt\.lib$') {
        foreach ($a in $Arch, 'x64') {
            $p = Join-Path $sdkLibRoot "ucrt\$a\$Name"
            if (Test-Path -LiteralPath $p) { return $p }
        }
        throw "ucrt.lib not found for arch $Arch"
    }
    # SDK um libs (kernel32, ntdll, etc.)
    foreach ($a in $Arch, 'x64', 'arm64') {
        $dir = Join-Path $sdkLibRoot "um\$a"
        if (Test-Path -LiteralPath $dir) {
            $found = Get-ChildItem $dir -Filter $Name -ErrorAction SilentlyContinue |
                Select-Object -First 1 -ExpandProperty FullName
            if ($found) { return $found }
        }
    }
    throw "SDK um lib $Name not found for arch $Arch"
}

$crtLibNames = @('vcruntime.lib')
$umLibNames  = @('kernel32.Lib', 'ntdll.lib', 'UserEnv.Lib', 'WS2_32.Lib', 'DbgHelp.Lib')

$importLibs = @()
foreach ($name in ($crtLibNames + $umLibNames)) {
    $importLibs += (Resolve-LibForView -Name $name -Arch 'arm64')
}
foreach ($name in ($crtLibNames + $umLibNames)) {
    $importLibs += (Resolve-LibForView -Name $name -Arch 'arm64ec')
}
# softintrin is EC (arm64ec/x64-ABI) only - SDK um x64
$softintrin = Join-Path $sdkLibRoot 'um\x64\softintrin.lib'
if (-not (Test-Path -LiteralPath $softintrin)) {
    $softintrin = Join-Path $sdkLibRoot 'um\arm64\softintrin.lib'
    if (-not (Test-Path -LiteralPath $softintrin)) { throw "softintrin.lib not found" }
}
$importLibs += $softintrin

# arm64\msvcrt.lib provides __icall_helper_arm64ec (needed by EC raw_dylib stubs)
# but also contains dll_dllmain_stub.obj which would override our DllMain if passed
# as an explicit input. Pass it via /DEFAULTLIB: so it is lower priority than the
# explicit staticlibs: our DllMain objects from rd_pipe.lib win, dll_dllmain_stub
# becomes the second definition (ignored by /force:multiple).
$msvcrtArm64Lib  = Join-Path $msvcLibRoot 'arm64\msvcrt.lib'
$msvcrtX64Lib    = Join-Path $msvcLibRoot 'x64\msvcrt.lib'
if (-not (Test-Path -LiteralPath $msvcrtArm64Lib)) { throw "arm64\msvcrt.lib not found at $msvcrtArm64Lib" }
if (-not (Test-Path -LiteralPath $msvcrtX64Lib))   { throw "x64\msvcrt.lib not found at $msvcrtX64Lib" }

Write-Host "==> Import libs (arm64 native + arm64ec EC):"
$importLibs | ForEach-Object { Write-Host "    $_" }

$outDll = Join-Path $OutDir 'rd_pipe.dll'

Write-Host "==> Linking ARM64X merged DLL with MSVC link.exe"
# /defArm64Native + /def supply the export tables for the ARM64 native and
# ARM64EC views respectively. Each DEF is generated from the matching
# per-arch DLL so the merged binary exposes the exact union of exports
# rustc emitted for each arch.
#
# /force:multiple: resolves duplicate DllMain symbols. The merged link has two
# DllMain definitions: one from each per-arch staticlib AND a stub from
# msvcrt.lib(dll_dllmain_stub.obj). The stub wins via /force:multiple but that
# is intentional: the stub is the CRT-wrapped DllMain — it calls
# dllmain_crt_process_attach which chains through _pRawDllMain to user code.
# Our actual DllMain (tracing init + Tokio runtime) runs via _pRawDllMain.
#
# arm64\msvcrt.lib is passed after the staticlibs via /NODEFAULTLIB + explicit
# path so the linker sees our DllMain objects before the stub. msvcrt.lib is
# still needed as an explicit input because arm64\msvcrt.lib provides
# __icall_helper_arm64ec (used by the EC view's raw_dylib import stubs) and
# arm64\vcruntime.lib provides __CxxFrameHandler3/__chkstk.
#
# /LIBPATH: dirs allow the .drectve /defaultlib:... entries baked into the
# staticlibs to resolve without vcvars being active.
$libpathMsvcArm64  = Join-Path $msvcLibRoot 'arm64'
$libpathMsvcX64    = Join-Path $msvcLibRoot 'x64'
$libpathUcrtArm64  = Join-Path $sdkLibRoot 'ucrt\arm64'
$libpathUcrtArm64ec= Join-Path $sdkLibRoot 'ucrt\arm64ec'
$libpathUcrtX64    = Join-Path $sdkLibRoot 'ucrt\x64'
$libpathUmArm64    = Join-Path $sdkLibRoot 'um\arm64'
$libpathUmX64      = Join-Path $sdkLibRoot 'um\x64'

& $linkExe `
    /dll /machine:arm64x /nologo `
    /noimplib `
    /force:multiple `
    "/NODEFAULTLIB:msvcrt" `
    "/LIBPATH:$libpathMsvcArm64" `
    "/LIBPATH:$libpathMsvcX64" `
    "/LIBPATH:$libpathUcrtArm64" `
    "/LIBPATH:$libpathUcrtArm64ec" `
    "/LIBPATH:$libpathUcrtX64" `
    "/LIBPATH:$libpathUmArm64" `
    "/LIBPATH:$libpathUmX64" `
    "/defArm64Native:$defArm64" `
    "/def:$defArm64ec" `
    "/out:$outDll" `
    $Arm64Lib $Arm64ecLib `
    @importLibs `
    $msvcrtArm64Lib $msvcrtX64Lib
if ($LASTEXITCODE -ne 0) { throw "MSVC link.exe failed with exit code $LASTEXITCODE" }

Write-Host "==> Merged ARM64X DLL built: $outDll"
Get-Item $outDll | Format-List FullName, Length, LastWriteTime

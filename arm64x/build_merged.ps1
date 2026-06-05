# Build a merged ARM64X DLL from the aarch64-pc-windows-msvc and
# arm64ec-pc-windows-msvc staticlibs of rd_pipe.
#
#   -Arm64Lib   : path to target\aarch64-pc-windows-msvc\release\rd_pipe.lib
#   -Arm64ecLib : path to target\arm64ec-pc-windows-msvc\release\rd_pipe.lib
#   -Arm64Dll   : path to target\aarch64-pc-windows-msvc\release\rd_pipe.dll
#   -Arm64ecDll : path to target\arm64ec-pc-windows-msvc\release\rd_pipe.dll
#   -OutDir     : directory to place the merged rd_pipe.dll
#
# Requires a VS Developer Command Prompt environment (vcvarsall arm64 or the
# ilammy/msvc-dev-cmd action with arch:arm64). This sets:
#   VCToolsInstallDir -> MSVC tool + lib root
#   WindowsSdkDir + WindowsSDKVersion -> SDK lib root
#   LIB               -> arm64 MSVC + arm64 ucrt + arm64 um (native view)
#
# link.exe and dumpbin.exe are resolved directly from VCToolsInstallDir\bin\HostX64\x64
# to avoid two PATH hazards on GitHub-hosted Windows runners:
#   - vcvarsall x64_arm64 only prepends HostX64\arm64, leaving dumpbin off PATH.
#   - Git for Windows ships usr\bin\link.exe (a Unix tool) before MSVC on PATH.
#
# Uses MSVC link.exe (/machine:arm64x). lld-link is NOT used: it corrupts the
# EC view's TLS directory when merging two Rust staticlibs into one ARM64X image.
#
# DEF files generated via dumpbin /exports; no llvm-tools component needed.

[CmdletBinding()]
param(
    [Parameter(Mandatory)] [string] $Arm64Lib,
    [Parameter(Mandatory)] [string] $Arm64ecLib,
    [Parameter(Mandatory)] [string] $Arm64Dll,
    [Parameter(Mandatory)] [string] $Arm64ecDll,
    [Parameter(Mandatory)] [string] $OutDir
)

$ErrorActionPreference = 'Stop'

# Validate vcvars environment.
foreach ($v in 'VCToolsInstallDir', 'WindowsSdkDir', 'WindowsSDKVersion') {
    if (-not (Get-Item "Env:$v" -ErrorAction SilentlyContinue)) {
        throw "$v not set — run inside a VS Developer Command Prompt (vcvarsall arm64)."
    }
}

$msvcLib = Join-Path $env:VCToolsInstallDir.TrimEnd('\/') 'lib'
$sdkLib  = Join-Path $env:WindowsSdkDir.TrimEnd('\/') "Lib\$($env:WindowsSDKVersion.TrimEnd('\/'))"

Write-Host "==> MSVC lib root:    $msvcLib"
Write-Host "==> Windows SDK root: $sdkLib"

# dumpbin.exe and link.exe must be resolved via VCToolsInstallDir rather than
# relying on PATH. Two PATH hazards on GitHub-hosted runners:
#
#   1. vcvarsall x64_arm64 (ilammy/msvc-dev-cmd arch:arm64) prepends
#      HostX64\arm64 but NOT HostX64\x64, so dumpbin.exe is missing from PATH.
#
#   2. Git for Windows ships C:\Program Files\Git\usr\bin\link.exe (a Unix
#      hard-link tool) earlier on PATH than any MSVC entry, so bare "link.exe"
#      resolves to the wrong binary.
#
# HostX64\x64\{dumpbin,link}.exe is always present for an x64 VS install and
# supports all /machine targets including arm64x.
$vcBinX64   = Join-Path $env:VCToolsInstallDir "bin\HostX64\x64"
$dumpbinExe = Join-Path $vcBinX64 "dumpbin.exe"
$linkExe    = Join-Path $vcBinX64 "link.exe"
foreach ($exe in $dumpbinExe, $linkExe) {
    if (-not (Test-Path $exe)) { throw "$exe not found — check VCToolsInstallDir" }
}
Write-Host "==> dumpbin: $dumpbinExe"
Write-Host "==> link:    $linkExe"

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

# Generate a DEF file from a per-arch DLL via dumpbin /exports.
#
# DllMain is the DLL entry point — it is not a named export and must not appear
# in the DEF file (causes LNK4006 duplicate-symbol warning against msvcrt's stub).
#
# DllGetClassObject and DllInstall are COM/regsvr32 entry points looked up via
# GetProcAddress; they must be PRIVATE (no import-lib entry) to suppress LNK4104.
$privateExports = @('DllGetClassObject', 'DllInstall')
$skipExports    = @('DllMain')

function New-DefFromDll {
    param([string]$Dll, [string]$DefPath)
    $names = & $dumpbinExe /exports $Dll 2>&1 |
        Select-String '^\s+\d+\s+[0-9A-Fa-f]+\s+[0-9A-Fa-f]+\s+(\w+)' |
        ForEach-Object { $_.Matches[0].Groups[1].Value } |
        Where-Object { $_ -notin $skipExports }
    if (-not $names) { throw "No exports found in $Dll" }
    $lines = $names | ForEach-Object {
        if ($_ -in $privateExports) { "    $_ PRIVATE" } else { "    $_" }
    }
    Set-Content -Path $DefPath -Value (@('EXPORTS') + $lines) -Encoding ASCII
    Write-Host "==> $DefPath ($($names.Count) exports): $($names -join ', ')"
}

$defArm64   = Join-Path $OutDir 'rd_pipe.arm64.def'
$defArm64ec = Join-Path $OutDir 'rd_pipe.arm64ec.def'
New-DefFromDll -Dll $Arm64Dll   -DefPath $defArm64
New-DefFromDll -Dll $Arm64ecDll -DefPath $defArm64ec

# Import libs for the /machine:arm64x link.
#
# arm64 (native) view: LIB env covers these; passed by bare name.
#
# arm64ec (EC) view: ARM64EC uses x64 calling convention, so the SDK stores
#   its import libs under the x64 subdirectory (no separate arm64ec dir).
#   MSVC CRT helpers (vcruntime, msvcrt) use the arm64 dir — same native
#   code runs for both views. Passed by full path since LIB is arm64-only.
#   arm64\msvcrt.lib provides __icall_helper_arm64ec for EC raw_dylib stubs.
#   softintrin.lib (arm64ec view only) is under SDK um\x64.
$importLibs = @(
    # arm64 native view — resolved via LIB env
    'vcruntime.lib',
    'kernel32.lib',
    'ntdll.lib',
    'userenv.lib',
    'ws2_32.lib',
    'dbghelp.lib',
    # arm64ec view — SDK arm64ec libs live under x64 subdir; MSVC CRT from arm64 dir
    "$msvcLib\arm64\vcruntime.lib",
    "$sdkLib\um\x64\kernel32.Lib",
    "$sdkLib\um\x64\ntdll.lib",
    "$sdkLib\um\x64\UserEnv.Lib",
    "$sdkLib\um\x64\WS2_32.Lib",
    "$sdkLib\um\x64\DbgHelp.Lib",
    "$sdkLib\um\x64\softintrin.lib"
)

# msvcrt.lib passed after staticlibs (via /NODEFAULTLIB + explicit path) so
# the linker processes our DllMain objects first. The msvcrt stub DllMain wins
# via /force:multiple but correctly chains through _pRawDllMain to our DllMain.
# arm64\msvcrt.lib serves both views: it provides __icall_helper_arm64ec for
# the arm64ec raw_dylib stubs as well as the native CRT startup glue.
$msvcrtLib = "$msvcLib\arm64\msvcrt.lib"

$outDll = Join-Path $OutDir 'rd_pipe.dll'

Write-Host "==> Linking ARM64X DLL..."
& $linkExe `
    /dll /machine:arm64x /nologo /noimplib /force:multiple `
    "/ignore:4001,4088" `
    "/NODEFAULTLIB:msvcrt" `
    "/LIBPATH:$msvcLib\arm64" `
    "/LIBPATH:$sdkLib\ucrt\arm64" `
    "/LIBPATH:$sdkLib\um\arm64" `
    "/LIBPATH:$sdkLib\ucrt\x64" `
    "/LIBPATH:$sdkLib\um\x64" `
    "/defArm64Native:$defArm64" "/def:$defArm64ec" `
    "/out:$outDll" `
    $Arm64Lib $Arm64ecLib @importLibs $msvcrtLib
if ($LASTEXITCODE -ne 0) { throw "link.exe failed ($LASTEXITCODE)" }

Write-Host "==> Built: $outDll"
Get-Item $outDll | Format-List FullName, Length, LastWriteTime

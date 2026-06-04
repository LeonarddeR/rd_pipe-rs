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
#   PATH              -> link.exe, dumpbin.exe
#   VCToolsInstallDir -> MSVC lib root
#   WindowsSdkDir + WindowsSDKVersion -> SDK lib root
#   LIB               -> arm64 MSVC + arm64 ucrt + arm64 um (native view)
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

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

# Generate a DEF file from a per-arch DLL via dumpbin /exports.
function New-DefFromDll {
    param([string]$Dll, [string]$DefPath)
    $names = dumpbin /exports $Dll 2>&1 |
        Select-String '^\s+\d+\s+[0-9A-Fa-f]+\s+[0-9A-Fa-f]+\s+(\w+)' |
        ForEach-Object { $_.Matches[0].Groups[1].Value }
    if (-not $names) { throw "No exports found in $Dll" }
    Set-Content -Path $DefPath -Value (@('EXPORTS') + ($names | ForEach-Object { "    $_" })) -Encoding ASCII
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
link.exe `
    /dll /machine:arm64x /nologo /noimplib /force:multiple `
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

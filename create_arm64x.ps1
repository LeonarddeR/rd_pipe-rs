# Script to combine ARM64 and ARM64EC static libraries into an ARM64X DLL
# Requires LLVM 22+ with lld-link
#
# Usage:
#   .\create_arm64x.ps1 -Arm64LibPath <path-to-arm64-lib> -Arm64EcLibPath <path-to-arm64ec-lib> -OutputPath <output-path>
#
# Example:
#   .\create_arm64x.ps1 -Arm64LibPath .\target\aarch64-pc-windows-msvc\release\rd_pipe.lib -Arm64EcLibPath .\target\arm64ec-pc-windows-msvc\release\rd_pipe.lib -OutputPath .\rd_pipe_arm64x.dll

param(
    [Parameter(Mandatory=$true)]
    [string]$Arm64LibPath,

    [Parameter(Mandatory=$true)]
    [string]$Arm64EcLibPath,

    [Parameter(Mandatory=$true)]
    [string]$OutputPath
)

# Validate input files exist
if (-not (Test-Path $Arm64LibPath)) {
    Write-Error "ARM64 library not found at: $Arm64LibPath"
    exit 1
}

if (-not (Test-Path $Arm64EcLibPath)) {
    Write-Error "ARM64EC library not found at: $Arm64EcLibPath"
    exit 1
}

Write-Host "Combining ARM64X library from:"
Write-Host "  ARM64:   $Arm64LibPath"
Write-Host "  ARM64EC: $Arm64EcLibPath"
Write-Host "  Output:  $OutputPath"

# Check if lld-link is available
$lldLink = Get-Command lld-link -ErrorAction SilentlyContinue
if (-not $lldLink) {
    Write-Error "lld-link not found. Please ensure LLVM 22+ is installed and in PATH."
    exit 1
}

Write-Host "Using lld-link from: $($lldLink.Source)"

# Create the ARM64X DLL using lld-link
# The /machine:arm64x flag creates a hybrid ARM64X image
# The input libs should be: ARM64 static lib first, then ARM64EC static lib
$lldLinkArgs = @(
    "/dll",
    "/machine:arm64x",
    "/out:$OutputPath",
    $Arm64LibPath,
    $Arm64EcLibPath,
    "/nologo"
)

Write-Host "Running: lld-link $($lldLinkArgs -join ' ')"

try {
    & lld-link $lldLinkArgs
    if ($LASTEXITCODE -ne 0) {
        Write-Error "lld-link failed with exit code $LASTEXITCODE"
        exit $LASTEXITCODE
    }

    Write-Host "Successfully created ARM64X library: $OutputPath" -ForegroundColor Green

    # Display file info
    if (Test-Path $OutputPath) {
        $fileInfo = Get-Item $OutputPath
        Write-Host "File size: $($fileInfo.Length) bytes"
    }
} catch {
    Write-Error "Failed to create ARM64X library: $_"
    exit 1
}

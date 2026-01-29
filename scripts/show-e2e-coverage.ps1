# E2E test coverage report script

$ErrorActionPreference = "Stop"

$CoverageFile = "coverage-e2e\coverage-data.json"

if (-not (Test-Path $CoverageFile)) {
    Write-Host "Coverage data file not found: $CoverageFile" -ForegroundColor Yellow
    Write-Host "Run E2E tests to collect coverage data." -ForegroundColor Gray
    Write-Host "Command: npm run test:e2e" -ForegroundColor Gray
    exit 0
}

Write-Host "Loading E2E test coverage results..." -ForegroundColor Cyan

try {
    $coverageData = Get-Content $CoverageFile -Encoding UTF8 | ConvertFrom-Json
    
    $totalFiles = $coverageData.Count
    $totalFunctions = 0
    $coveredFunctions = 0
    
    foreach ($file in $coverageData) {
        if ($file.functions -and $file.functions.Count -gt 0) {
            foreach ($func in $file.functions) {
                $totalFunctions++
                $hasCoverage = $false
                if ($func.ranges -and $func.ranges.Count -gt 0) {
                    foreach ($range in $func.ranges) {
                        if ($range.count -gt 0) {
                            $hasCoverage = $true
                            break
                        }
                    }
                }
                if ($hasCoverage) {
                    $coveredFunctions++
                }
            }
        }
    }
    
    $coveragePercentage = if ($totalFunctions -gt 0) {
        [math]::Round(($coveredFunctions / $totalFunctions) * 100, 2)
    } else {
        0
    }
    
    Write-Host ""
    Write-Host "E2E Test Coverage Summary:" -ForegroundColor Green
    Write-Host ""
    Write-Host "  Total Files: $totalFiles" -ForegroundColor Cyan
    Write-Host "  Total Functions: $totalFunctions" -ForegroundColor Cyan
    Write-Host "  Covered Functions: $coveredFunctions" -ForegroundColor Cyan
    Write-Host "  Coverage: $coveragePercentage%" -ForegroundColor $(if ($coveragePercentage -ge 50) { "Green" } else { "Yellow" })
    Write-Host ""
    
    # File-by-file coverage
    if ($totalFiles -gt 0) {
        Write-Host "File-by-file Coverage:" -ForegroundColor Cyan
        Write-Host ""
        $fileCount = 0
        foreach ($file in $coverageData) {
            $fileCount++
            if ($fileCount -gt 20) {
                Write-Host "  ... and $($totalFiles - 20) more files" -ForegroundColor Gray
                break
            }
            $url = $file.url
            if ($url -match 'http://localhost:1420/(.+)') {
                $url = $matches[1]
            } elseif ($url -match 'http://localhost:1420') {
                $url = "index"
            }
            $funcCount = if ($file.functions) { $file.functions.Count } else { 0 }
            $coveredCount = 0
            if ($file.functions) {
                foreach ($func in $file.functions) {
                    if ($func.ranges) {
                        foreach ($range in $func.ranges) {
                            if ($range.count -gt 0) {
                                $coveredCount++
                                break
                            }
                        }
                    }
                }
            }
            $fileCoverage = if ($funcCount -gt 0) {
                [math]::Round(($coveredCount / $funcCount) * 100, 1)
            } else {
                0
            }
            Write-Host "  $url : $fileCoverage% ($coveredCount/$funcCount)" -ForegroundColor Gray
        }
    }
    
    Write-Host ""
    Write-Host "Note: Playwright built-in coverage is JS/CSS coverage only." -ForegroundColor Yellow
    Write-Host "For detailed coverage, vite-plugin-istanbul is required." -ForegroundColor Yellow
    
} catch {
    Write-Host "Failed to load coverage data: $_" -ForegroundColor Red
    exit 1
}

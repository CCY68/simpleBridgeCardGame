<#
.SYNOPSIS
    CardArena Local Demo Script (Windows PowerShell)

.DESCRIPTION
    啟動 Server + 1 Human CLI + 3 AI Clients 進行完整遊戲

.PARAMETER Port
    Server port (default: 8888)

.PARAMETER Humans
    Number of human players (default: 1, max: 4)

.PARAMETER Seed
    Random seed for dealing (optional)

.PARAMETER NoBuild
    Skip cargo build

.EXAMPLE
    .\run_local_demo.ps1
    .\run_local_demo.ps1 -Port 9999 -Humans 2
#>

param(
    [int]$Port = 8888,
    [int]$Humans = 1,
    [string]$Seed = "",
    [switch]$NoBuild
)

$ErrorActionPreference = "Stop"

# 計算 AI 數量
$AICount = 4 - $Humans

if ($Humans -lt 1 -or $Humans -gt 4) {
    Write-Error "Humans must be between 1 and 4"
    exit 1
}

# 取得專案根目錄
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir

Write-Host ""
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "   CardArena Local Demo (Windows)" -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Port:    $Port"
Write-Host "Humans:  $Humans"
Write-Host "AIs:     $AICount"
if ($Seed) { Write-Host "Seed:    $Seed" }
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""

# 清理函數
function Cleanup {
    Write-Host "[INFO] Cleaning up processes..." -ForegroundColor Green
    Get-Process -Name "card_arena_server" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Get-Process -Name "python*" -ErrorAction SilentlyContinue | Where-Object {
        $_.CommandLine -match "ai_cli|human_cli"
    } | Stop-Process -Force -ErrorAction SilentlyContinue
    Write-Host "[INFO] Cleanup complete." -ForegroundColor Green
}

# 註冊清理
$null = Register-EngineEvent PowerShell.Exiting -Action { Cleanup }

try {
    # Step 1: Build Server
    if (-not $NoBuild) {
        Write-Host "[INFO] Building server..." -ForegroundColor Green
        Push-Location "$ProjectRoot\server"
        cargo build --release
        Pop-Location
    }

    # Step 2: Start Server
    Write-Host "[INFO] Starting server on port $Port..." -ForegroundColor Green
    $env:RUST_LOG = "info"
    $ServerProcess = Start-Process -FilePath "cargo" `
        -ArgumentList "run", "--release", "--", "--port", $Port `
        -WorkingDirectory "$ProjectRoot\server" `
        -PassThru -WindowStyle Hidden

    Start-Sleep -Seconds 3

    if ($ServerProcess.HasExited) {
        Write-Error "Server failed to start!"
        exit 1
    }
    Write-Host "[INFO] Server started (PID: $($ServerProcess.Id))" -ForegroundColor Green

    # Step 3: Start AI Clients
    Write-Host "[INFO] Starting $AICount AI client(s)..." -ForegroundColor Green
    $AIProcesses = @()
    for ($i = 1; $i -le $AICount; $i++) {
        $proc = Start-Process -FilePath "python" `
            -ArgumentList "$ProjectRoot\clients\ai_cli\app.py", `
                "--host", "127.0.0.1", `
                "--port", $Port, `
                "--name", "Bot_$i", `
                "--token", "secret", `
                "--no-llm" `
            -PassThru -WindowStyle Hidden
        $AIProcesses += $proc
        Start-Sleep -Milliseconds 300
    }

    # Step 4: Start Human CLI Client
    if ($Humans -gt 0) {
        Write-Host "[INFO] Starting Human CLI client..." -ForegroundColor Green
        Write-Host ""
        Write-Host "======================================" -ForegroundColor Yellow
        Write-Host "  Human Client - Interactive Mode" -ForegroundColor Yellow
        Write-Host "======================================" -ForegroundColor Yellow
        Write-Host ""

        # 在當前視窗執行 human client
        python "$ProjectRoot\clients\human_cli\app.py" `
            --host 127.0.0.1 `
            --port $Port `
            --name "Player_1"

        if ($Humans -gt 1) {
            Write-Host "[WARN] For multiple human players, please open additional terminals and run:" -ForegroundColor Yellow
            for ($i = 2; $i -le $Humans; $i++) {
                Write-Host "  python clients\human_cli\app.py --host 127.0.0.1 --port $Port --name Player_$i"
            }
        }
    }
}
finally {
    Cleanup
}

Write-Host ""
Write-Host "[INFO] Demo session ended." -ForegroundColor Green

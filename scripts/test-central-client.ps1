param([int]$Monitor = 0)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = Split-Path $PSScriptRoot
$run = Join-Path $root ('target/central-ui-' + [IO.Path]::GetRandomFileName().Replace('.', ''))
[void][IO.Directory]::CreateDirectory($run)
$ready = Join-Path $run 'fixture.json'
$client = $null
$fixture = $null
$port = $null
$endpoint = $null
$fixtureOutput = $null
$fixtureErrors = $null
$clientOutput = $null
$clientErrors = $null

function Start-Owned([string]$Executable, [string[]]$Arguments, [hashtable]$Environment) {
    $start = [Diagnostics.ProcessStartInfo]::new()
    $start.FileName = $Executable
    $start.WorkingDirectory = $root
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    foreach ($argument in $Arguments) { $start.ArgumentList.Add($argument) }
    foreach ($key in $Environment.Keys) { $start.Environment[$key] = $Environment[$key] }
    return [Diagnostics.Process]::Start($start)
}
function Get-State { Invoke-RestMethod -Uri "$endpoint/state" -TimeoutSec 12 }
function Send-Control([string]$Route, [hashtable]$Body) {
    $reply = Invoke-RestMethod -Uri "$endpoint/$Route" -Method Post -ContentType application/json -Body ($Body | ConvertTo-Json -Compress) -TimeoutSec 15
    if ($reply.PSObject.Properties['error']) { throw "Harness refused $Route" }
    return $reply
}
function Click-Control([string]$Press) {
    $controls = @((Get-State).lobby_controls | Where-Object press -CEQ $Press)
    if ($controls.Count -ne 1) { throw "Expected one visible control: $Press" }
    Send-Control pointer @{x=$controls[0].at_x; y=$controls[0].at_y; button='left'; press=$true} | Out-Null
}
function Wait-Account([scriptblock]$Condition) {
    $deadline = [DateTime]::UtcNow.AddSeconds(35)
    $settled = 0
    do {
        $snapshot = Get-State
        if (-not $snapshot.account.front_moving -and (& $Condition $snapshot.account)) { $settled++ } else { $settled = 0 }
        if ($settled -ge 2) { return $snapshot }
        Start-Sleep -Milliseconds 250
    } while ([DateTime]::UtcNow -lt $deadline -and -not $client.HasExited)
    $snapshot | ConvertTo-Json -Depth 45 | Set-Content (Join-Path $run 'failure-state.json')
    throw 'Expected account state was not reached'
}

try {
    # Build separately before this script so the bounded backend handoff is used for UI work.
    $executable = Join-Path $root 'target/debug/baylee-client.exe'
    if (-not $IsWindows) { $executable = Join-Path $root 'target/debug/baylee-client' }
    if (-not (Test-Path -LiteralPath $executable)) { throw 'Build the dev-control client first' }
    $fixture = Start-Owned (Get-Process -Id $PID).Path @('-NoProfile','-File',(Join-Path $PSScriptRoot 'test-central-integration.ps1'),'-CancelFile',(Join-Path $run 'cancel')) @{BAYLEE_CENTRAL_UI_READY=$ready}
    $fixtureOutput = $fixture.StandardOutput.ReadToEndAsync()
    $fixtureErrors = $fixture.StandardError.ReadToEndAsync()
    $deadline = [DateTime]::UtcNow.AddSeconds(160)
    while (-not (Test-Path -LiteralPath $ready)) {
        if ($fixture.HasExited) { throw 'Synthetic backend fixture failed before UI handoff' }
        if ([DateTime]::UtcNow -ge $deadline) { throw 'Synthetic backend handoff timed out' }
        Start-Sleep -Milliseconds 250
    }
    $config = Get-Content -LiteralPath $ready -Raw | ConvertFrom-Json
    if ($config.mode -cne 'synthetic-only' -or $config.central -notmatch '^http://127\.0\.0\.1:\d+$' -or $config.gateway -notmatch '^http://127\.0\.0\.1:\d+$') { throw 'Unexpected fixture origins' }
    $listener = [Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback, 0)
    $listener.Start()
    try { $port = $listener.LocalEndpoint.Port } finally { $listener.Stop() }
    $endpoint = "http://127.0.0.1:$port"
    $configHome = Join-Path $run 'config'
    [void][IO.Directory]::CreateDirectory((Join-Path $configHome 'baylee'))
    '{"lang":"en","feedback_url":"","usage_count":false,"gateways":["https://unused-device.invalid"]}' | Set-Content (Join-Path $configHome 'baylee/settings.json')
    $client = Start-Owned $executable @() @{
        XDG_CONFIG_HOME=$configHome; XDG_CACHE_HOME=(Join-Path $run 'cache'); XDG_DATA_HOME=(Join-Path $run 'data')
        BAYLEE_DEV_CONTROL="$port"; BAYLEE_DEV_PASSIVE='1'; BAYLEE_DEV_MONITOR="$Monitor"; BAYLEE_DEV_WINDOW='1280x800'
        BAYLEE_DEV_SOUND=''; BAYLEE_NO_TOURS='1'; BAYLEE_CENTRAL_DEV_ENDPOINT=$config.central
        BAYLEE_GATEWAY=$config.gateway; BAYLEE_UPDATE_API="$($config.gateway)/synthetic-updates-disabled"
    }
    $clientOutput = $client.StandardOutput.ReadToEndAsync()
    $clientErrors = $client.StandardError.ReadToEndAsync()
    Write-Output "Owned client PID $($client.Id); artifacts $run"
    $deadline = [DateTime]::UtcNow.AddSeconds(45)
    do {
        try { $health = Invoke-RestMethod "$endpoint/health" -TimeoutSec 2; break } catch {
            if ($client.HasExited -or [DateTime]::UtcNow -ge $deadline) { throw 'Client harness did not start' }
            Start-Sleep -Milliseconds 250
        }
    } while ($true)
    $monitors = Invoke-RestMethod "$endpoint/monitors" -TimeoutSec 5
    $monitors | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $run 'monitors.json')
    $selected = @($monitors.monitors | Where-Object index -EQ $Monitor)
    if ($selected.Count -ne 1 -or $selected[0].x -ge 0) { throw 'Selected monitor is not the additional left display' }
    $deadline = [DateTime]::UtcNow.AddSeconds(40)
    while ((Get-State).loading.destination -cne 'hidden') {
        if ($client.HasExited -or [DateTime]::UtcNow -ge $deadline) { throw 'Client startup overlay did not close' }
        Start-Sleep -Milliseconds 250
    }
    $initial = Wait-Account { param($account) $account.central_pending -and -not $account.central_ready -and $account.saved_gateway_count -eq 0 }
    Send-Control screenshot @{path=(Join-Path $run '01-login.png')} | Out-Null
    Click-Control 'Shared(Focus(Username))'
    Send-Control text @{text='synthetic'} | Out-Null
    Click-Control 'Shared(Focus(Password))'
    Send-Control text @{text='a-very-fine-password'} | Out-Null
    Click-Control 'Front(Submit)'
    $restored = Wait-Account { param($account) $account.central_ready -and -not $account.central_pending -and $account.saved_gateway_count -eq 1 }
    Send-Control screenshot @{path=(Join-Path $run '02-gateways.png')} | Out-Null
    Click-Control 'Front(CentralSignOut)'
    $signedOut = Wait-Account { param($account) $account.central_pending -and -not $account.central_ready -and -not $account.gateway_authenticated -and $account.saved_gateway_count -eq 0 }
    Click-Control 'Shared(Focus(Username))'
    Send-Control text @{text='synthetic'} | Out-Null
    Click-Control 'Shared(Focus(Password))'
    Send-Control text @{text='a-very-fine-password'} | Out-Null
    Click-Control 'Front(Submit)'
    $restored = Wait-Account { param($account) $account.central_ready -and -not $account.central_pending -and $account.saved_gateway_count -eq 1 }
    Click-Control 'Front(SelectGateway(0))'
    $admitted = Wait-Account { param($account) $account.central_ready -and $account.gateway_authenticated -and -not $account.offline }
    Send-Control screenshot @{path=(Join-Path $run '03-admitted.png')} | Out-Null
    @{initial=$initial.account; restored=$restored.account; signed_out=$signedOut.account; admitted=$admitted.account} | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $run 'result.json')
    Write-Output 'Native Central login, restored gateway list and ticket admission passed.'
} finally {
    $cleanupErrors = [Collections.Generic.List[string]]::new()
    if ($null -ne $client) {
        try {
            if (-not $client.HasExited) { $client.Kill($true) }
            if (-not $client.WaitForExit(10000)) { throw 'Owned client remains' }
            if ($null -ne $clientOutput) { $clientOutput.GetAwaiter().GetResult() | Set-Content (Join-Path $run 'client.stdout.log') }
            if ($null -ne $clientErrors) { $clientErrors.GetAwaiter().GetResult() | Set-Content (Join-Path $run 'client.stderr.log') }
        } catch { $cleanupErrors.Add('Owned client cleanup failed') }
        finally { $client.Dispose() }
    }
    if ($null -ne $fixture) {
        try {
            [IO.File]::WriteAllText([IO.Path]::ChangeExtension($ready, 'stop'), 'stop')
            if (-not $fixture.WaitForExit(15000)) {
                [IO.File]::WriteAllText((Join-Path $run 'cancel'), 'cancel')
                if (-not $fixture.WaitForExit(30000)) { throw 'Backend fixture cleanup did not finish' }
            }
            $fixtureOutput.GetAwaiter().GetResult() | Set-Content (Join-Path $run 'fixture.stdout.log')
            $fixtureErrors.GetAwaiter().GetResult() | Set-Content (Join-Path $run 'fixture.stderr.log')
            if ($fixture.ExitCode -ne 0) { throw 'Backend fixture failed' }
        } catch { $cleanupErrors.Add('Backend fixture failed or cleanup is unverified; inspect its log') }
        finally { $fixture.Dispose() }
    }
    if ($null -ne $port) {
        $socket = [Net.Sockets.TcpClient]::new()
        try {
            $probe = $socket.ConnectAsync('127.0.0.1', $port)
            if (-not $probe.Wait(3000)) { throw 'Port closure probe timed out' }
            $cleanupErrors.Add('Owned client listener remains')
        } catch [AggregateException] {
            if ($_.Exception.GetBaseException() -isnot [Net.Sockets.SocketException] -or $_.Exception.GetBaseException().SocketErrorCode -ne [Net.Sockets.SocketError]::ConnectionRefused) { $cleanupErrors.Add('Client listener closure unverified') }
        } catch { $cleanupErrors.Add('Client listener closure unverified') }
        finally { $socket.Dispose() }
    }
    if ($cleanupErrors.Count -gt 0) { throw ($cleanupErrors -join '; ') }
    Write-Output 'Owned client exited; client port closed; backend fixture cleanup verified.'
}

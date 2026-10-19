param(
    [switch]$ExerciseFailure,
    [string]$CargoConfig,
    [ValidateSet('e2e_account', 'e2e_admin', 'e2e_admission', 'e2e_chair_tickets',
        'e2e_departure', 'e2e_lobby_feed', 'e2e_moderation', 'e2e_rematch',
        'e2e_rooms', 'e2e_seat_rate', 'e2e_social_depart', 'e2e_social_friends',
        'e2e_social_groups', 'e2e_social_invites', 'e2e_social_keys',
        'e2e_social_messages', 'e2e_social_presence', 'e2e_social_reports',
        'e2e_social_table', 'e2e_spectate', 'e2e_ws_tickets')]
    [string[]]$GatewayTests = @()
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Push-Location (Split-Path $PSScriptRoot)
# Official multi-architecture PostgreSQL 18 image, pinned rather than a moving tag.
$image = 'postgres@sha256:77f585114c32fbca283dc835b0596f4e52b51b4c6662d7810b2f4084f60a1873'
$name = 'baylee-central-db-' + [IO.Path]::GetRandomFileName().Replace('.', '')
$containerId = $null
$testProcess = $null
$port = $null
$adminSecret = [Convert]::ToHexString([Security.Cryptography.RandomNumberGenerator]::GetBytes(32))


$previousPassword = [Environment]::GetEnvironmentVariable('POSTGRES_PASSWORD')
function Restore-FixturePassword {
    # PowerShell passes plain $null as an empty string to this .NET overload.
    # NullString is required to preserve an originally absent variable.
    if ($null -eq $previousPassword) {
        [Environment]::SetEnvironmentVariable('POSTGRES_PASSWORD', [NullString]::Value)
    } else {
        [Environment]::SetEnvironmentVariable('POSTGRES_PASSWORD', $previousPassword)
    }
    if ([Environment]::GetEnvironmentVariable('POSTGRES_PASSWORD') -cne $previousPassword) {
        throw 'Temporary fixture environment was not restored exactly'
    }
}

try {
    $cargoOptions = @('--locked')
    if ($CargoConfig) {
        if (-not (Test-Path -LiteralPath $CargoConfig -PathType Leaf)) { throw 'Cargo config must be an existing file' }
        $cargoOptions = @('--config', (Resolve-Path -LiteralPath $CargoConfig).ProviderPath)
    }
    Write-Output "Owned PostgreSQL fixture: $name"
    & docker image inspect $image *> $null
    if ($LASTEXITCODE -ne 0) {
        & docker pull $image
        if ($LASTEXITCODE -ne 0) { throw 'Pinned PostgreSQL image unavailable' }
    }
    # Only this child shell gets the synthetic admin credential; no value in argv.
    [Environment]::SetEnvironmentVariable('POSTGRES_PASSWORD', $adminSecret)
    # Repeated gateway schema creation also generates WAL; keep the optional suite
    # bounded without exhausting the smaller database-only fixture's filesystem.
    $memory = if ($GatewayTests.Count -gt 0) { '2g' } else { '512m' }
    $dataMount = if ($GatewayTests.Count -gt 0) {
        '/var/lib/postgresql:rw,nosuid,nodev,size=1073741824'
    } else { '/var/lib/postgresql:rw,nosuid,nodev,size=268435456' }
    $created = & docker create --name $name --label baylee.central.test=owned --memory $memory --cpus 2 --pids-limit 128 --read-only --tmpfs $dataMount --tmpfs /var/run/postgresql:rw,nosuid,nodev --tmpfs /tmp:rw,nosuid,nodev --publish 127.0.0.1::5432 --env POSTGRES_PASSWORD --env PGDATA=/var/lib/postgresql/test $image 2>&1
    if ($LASTEXITCODE -ne 0) { throw 'Creating isolated PostgreSQL fixture failed' }
    Restore-FixturePassword
    $candidateId = ($created | Out-String).Trim()
    if ($candidateId -notmatch '^[0-9a-f]{64}$') { throw 'Invalid owned container identity' }
    $containerId = $candidateId
    & docker start $containerId | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Starting PostgreSQL fixture failed' }
    $ready = $false
    for ($attempt = 0; $attempt -lt 50; $attempt++) {
        & docker exec $containerId pg_isready -q -h 127.0.0.1 -U postgres *> $null
        if ($LASTEXITCODE -eq 0) { $ready = $true; break }
        Start-Sleep -Milliseconds 500
    }
    if (-not $ready) { throw 'PostgreSQL fixture did not become ready' }
    $portText = (& docker port $containerId 5432/tcp | Out-String).Trim()
    if ($LASTEXITCODE -ne 0 -or $portText -notmatch '^127\.0\.0\.1:(\d+)$') {
        throw 'Fixture must publish exclusively on IPv4 loopback'
    }
    $port = [int]$Matches[1]
    if ($ExerciseFailure) { throw 'Synthetic fixture failure' }
    $version = (& docker exec $containerId psql -X -At -U postgres -c 'SHOW server_version' | Out-String).Trim()
    Write-Output "Testing real PostgreSQL $version in an isolated synthetic Baylee database."
    $suites = @(@{ Package = 'baylee-db'; Target = 'schema' })
    foreach ($target in ($GatewayTests | Select-Object -Unique)) {
        $suites += @{ Package = 'baylee-gateway'; Target = $target }
    }
    foreach ($suite in $suites) {
        Write-Output "Testing $($suite.Package)/$($suite.Target)"
        $start = [Diagnostics.ProcessStartInfo]::new()
        $start.FileName = 'cargo'
        $start.UseShellExecute = $false
        $start.CreateNoWindow = $true
        $start.RedirectStandardOutput = $true
        $start.RedirectStandardError = $true
        [void]$start.Environment.Remove('POSTGRES_PASSWORD')
        [void]$start.Environment.Remove('PGPASSWORD')
        [void]$start.Environment.Remove('DATABASE_URL')
        foreach ($argument in (@('test') + $cargoOptions + @('-p', $suite.Package, '--test', $suite.Target, '--', '--test-threads=1'))) {
            $start.ArgumentList.Add($argument)
        }
        $start.Environment['DATABASE_URL'] = "postgres://postgres:${adminSecret}@127.0.0.1:${port}/postgres"
        $testProcess = [Diagnostics.Process]::Start($start)
        $testOutput = $testProcess.StandardOutput.ReadToEndAsync()
        $testErrors = $testProcess.StandardError.ReadToEndAsync()
        if (-not $testProcess.WaitForExit(300000)) { throw 'PostgreSQL integration tests timed out' }
        $resultText = $testOutput.GetAwaiter().GetResult()
        $errorText = $testErrors.GetAwaiter().GetResult()
        Write-Output $resultText
        if ($testProcess.ExitCode -ne 0) { Write-Output $errorText; throw 'PostgreSQL integration tests failed' }
        if ($resultText -notmatch 'test result: ok\. [1-9][0-9]* passed; 0 failed; 0 ignored;') {
            throw 'Expected PostgreSQL test suite did not run completely'
        }
        if ($suite.Target -eq 'schema' -and
            $resultText -notmatch 'test central_cutover_preserves_sources_reconciles_snapshots_and_revokes_legacy_sessions \.\.\. ok') {
            throw 'Expected central cutover test did not run'
        }
        $testProcess.Dispose()
        $testProcess = $null
        # Reclaim generated fixture WAL between suites, never while a child is active.
        & docker exec $containerId psql -X -q -U postgres -c 'CHECKPOINT' *> $null
        if ($LASTEXITCODE -ne 0) { throw 'Fixture checkpoint failed' }
    }
} finally {
    $cleanupErrors = [Collections.Generic.List[string]]::new()
    try {
        if ($null -ne $testProcess) {
            try {
                if (-not $testProcess.HasExited) { $testProcess.Kill($true) }
                if (-not $testProcess.WaitForExit(10000)) { throw 'process remains' }
            } finally { $testProcess.Dispose() }
        }
    } catch { $cleanupErrors.Add('Owned integration process cleanup could not be verified') }
    try { Restore-FixturePassword }
    catch { $cleanupErrors.Add('Temporary fixture environment could not be restored') }
    try {
        # The preallocated random name also handles an interruption during create.
        $found = & docker ps -a --no-trunc --filter "name=^/$name$" --format '{{.ID}} {{.Label "baylee.central.test"}}' 2>$null
        if ($LASTEXITCODE -ne 0) { throw 'container inventory unavailable' }
        $foundText = ($found | Out-String).Trim()
        if ($foundText.Length -gt 0) {
            if ($foundText -notmatch '^([0-9a-f]{64}) owned$') { throw 'ownership marker mismatch' }
            $ownedId = $Matches[1]
            if ($null -ne $containerId -and $ownedId -ne $containerId) { throw 'container identity changed' }
            & docker rm --force --volumes $ownedId | Out-Null
            if ($LASTEXITCODE -ne 0) { throw 'container removal failed' }
            $remaining = & docker ps -a --no-trunc --filter "id=$ownedId" --format '{{.ID}}' 2>$null
            if ($LASTEXITCODE -ne 0 -or ($remaining | Out-String).Trim().Length -gt 0) {
                throw 'container removal unverified'
            }
        }
    } catch { $cleanupErrors.Add('Owned PostgreSQL container cleanup could not be verified') }
    try {
        if ($null -ne $port) {
            $socket = [Net.Sockets.TcpClient]::new()
            try {
                try {
                    $task = $socket.ConnectAsync('127.0.0.1', $port)
                    if (-not $task.Wait(5000)) { throw 'listener probe timed out' }
                    throw 'listener remains'
                } catch [AggregateException] {
                    $cause = $_.Exception.GetBaseException()
                    if ($cause -isnot [Net.Sockets.SocketException] -or
                        $cause.SocketErrorCode -ne [Net.Sockets.SocketError]::ConnectionRefused) { throw }
                }
            } finally { $socket.Dispose() }
        }
    } catch { $cleanupErrors.Add('Owned PostgreSQL listener closure could not be verified') }
    Pop-Location
    if ($cleanupErrors.Count -gt 0) { throw ($cleanupErrors -join '; ') }
}
Write-Output 'PostgreSQL fixture passed and its owned container/listener were removed.'
exit 0

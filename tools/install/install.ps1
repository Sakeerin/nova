# Install nova, the Nova toolchain, on Windows from a GitHub release: the
# x86_64-pc-windows-msvc archive, checked against the release's
# SHA256SUMS, into $env:NOVA_HOME\bin (by default %USERPROFILE%\.nova\bin),
# which it adds to the user's PATH. Running it again updates nova;
# deleting %USERPROFILE%\.nova uninstalls it. Spec:
# docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md
# section 6.
#
#   irm https://github.com/Sakeerin/nova/releases/latest/download/install.ps1 | iex
#
# Environment:
#   NOVA_VERSION          the version to install, such as 0.3.0
#                         (default: the latest release)
#   NOVA_HOME             nova's home; nova goes into its bin directory
#                         (default: %USERPROFILE%\.nova)
#   NOVA_INSTALL_BASE     where the releases are
#                         (default: https://github.com/Sakeerin/nova/releases)
#   NOVA_INSTALL_NO_PATH  1 leaves PATH alone
#
# It runs in Windows PowerShell 5.1 and PowerShell 7, and is plain ASCII,
# because 5.1 reads a script without a byte-order mark in the system's
# code page. Everything happens in one script block, so `iex` leaves
# nothing behind in the session. An error goes to stderr and exits 1 when
# the script runs as a file; through `iex` at a prompt it is thrown
# instead, so the window stays open.

& {
    Set-StrictMode -Version Latest
    $ErrorActionPreference = 'Stop'
    # Invoke-WebRequest's progress bar slows 5.1's downloads many times over.
    $ProgressPreference = 'SilentlyContinue'

    function Say([string] $Message) {
        Write-Host "nova: $Message"
    }

    # This system's release target (spec section 6.1, step 1). A 32-bit
    # PowerShell on 64-bit Windows sees x86 in PROCESSOR_ARCHITECTURE and
    # the machine's AMD64 in PROCESSOR_ARCHITEW6432.
    function Get-NovaTarget {
        $os = $env:OS
        $arch = $env:PROCESSOR_ARCHITEW6432
        if (-not $arch) { $arch = $env:PROCESSOR_ARCHITECTURE }
        if ($os -eq 'Windows_NT' -and $arch -eq 'AMD64') {
            return 'x86_64-pc-windows-msvc'
        }
        if (-not $os) { $os = [Environment]::OSVersion.Platform.ToString() }
        if (-not $arch) { $arch = 'unknown' }
        throw "no ready-built nova for $os $arch; install it with: cargo install nova-cli --locked"
    }

    # The latest release's version, from where <base>/latest redirects:
    # .../releases/tag/v<version> (step 2). The redirect is read, not
    # followed.
    function Get-LatestVersion([string] $Base) {
        Add-Type -AssemblyName System.Net.Http
        $handler = New-Object System.Net.Http.HttpClientHandler
        $handler.AllowAutoRedirect = $false
        $client = New-Object System.Net.Http.HttpClient($handler)
        try {
            $response = $client.GetAsync("$Base/latest").GetAwaiter().GetResult()
            $location = $response.Headers.Location
            $status = [int] $response.StatusCode
            $response.Dispose()
        } catch {
            throw "could not reach $Base/latest: $($_.Exception.GetBaseException().Message)"
        } finally {
            $client.Dispose()
        }
        if ($null -eq $location) {
            throw "could not tell the latest version: $Base/latest answered $status, not a redirect"
        }
        $text = $location.OriginalString
        if ($text -notmatch '/tag/([^/?#]+)') {
            throw "could not tell the latest version: $Base/latest redirects to $text"
        }
        return $Matches[1]
    }

    function Save-Download([string] $Url, [string] $Path) {
        Say "downloading $Url"
        try {
            Invoke-WebRequest -Uri $Url -OutFile $Path -UseBasicParsing
        } catch {
            throw "could not download ${Url}: $($_.Exception.GetBaseException().Message)"
        }
    }

    # The same directory, whatever the case and a trailing backslash, and
    # with any %VARIABLE% in the PATH entry expanded.
    function Test-SameDirectory([string] $Entry, [string] $Dir) {
        $expanded = [Environment]::ExpandEnvironmentVariables($Entry).TrimEnd('\')
        return [string]::Equals($expanded, $Dir.TrimEnd('\'), [StringComparison]::OrdinalIgnoreCase)
    }

    # Step 7: the user's PATH, as the registry holds it, and this session's.
    # The value is read with its %VARIABLES% unexpanded and written back as
    # REG_EXPAND_SZ, as rustup does; [Environment]::SetEnvironmentVariable
    # would store every entry expanded, as REG_SZ.
    function Add-ToUserPath([string] $Dir) {
        $key = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey('Environment')
        try {
            $current = [string] $key.GetValue('Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
            $entries = @($current -split ';' | Where-Object { $_ -ne '' })
            $present = @($entries | Where-Object { Test-SameDirectory $_ $Dir }).Count -gt 0
            if ($present) {
                Say "$Dir is on your PATH already"
            } else {
                $updated = (@($Dir) + $entries) -join ';'
                $key.SetValue('Path', $updated, [Microsoft.Win32.RegistryValueKind]::ExpandString)
                # Setting a user variable through [Environment] tells running
                # programs, Explorer among them, to read the environment
                # again; this one is only removed, as it never exists.
                [Environment]::SetEnvironmentVariable('NOVA_INSTALL_PATH_CHANGED', $null, 'User')
                Say "added $Dir to your PATH; open a new terminal for other programs to see it"
            }
        } finally {
            $key.Close()
        }
        $session = @($env:Path -split ';' | Where-Object { $_ -ne '' })
        if (@($session | Where-Object { Test-SameDirectory $_ $Dir }).Count -eq 0) {
            $env:Path = "$Dir;$env:Path"
        }
    }

    function Install-Nova {
        # Windows PowerShell 5.1 may not offer TLS 1.2, which GitHub needs.
        [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

        $target = Get-NovaTarget
        $base = $env:NOVA_INSTALL_BASE
        if (-not $base) { $base = 'https://github.com/Sakeerin/nova/releases' }
        $base = $base.TrimEnd('/')
        $novaHome = $env:NOVA_HOME
        if (-not $novaHome) {
            if (-not $env:USERPROFILE) { throw 'neither NOVA_HOME nor USERPROFILE is set' }
            $novaHome = Join-Path $env:USERPROFILE '.nova'
        }
        $bin = Join-Path $novaHome 'bin'

        $version = $env:NOVA_VERSION
        if (-not $version) {
            Say "finding the latest release at $base/latest"
            $version = Get-LatestVersion $base
        }
        $version = $version -replace '^v', ''
        if ($version -notmatch '^[0-9A-Za-z.+-]+$') { throw "'$version' is not a version" }
        $name = "nova-$version-$target"
        $archive = "$name.zip"
        Say "installing nova $version for $target"

        $tmp = Join-Path ([IO.Path]::GetTempPath()) ('nova-install-' + [Guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Path $tmp | Out-Null
        $staged = $null
        try {
            # Step 3: the archive and SHA256SUMS, into the temporary directory.
            $zip = Join-Path $tmp $archive
            $sums = Join-Path $tmp 'SHA256SUMS'
            Save-Download "$base/download/v$version/$archive" $zip
            Save-Download "$base/download/v$version/SHA256SUMS" $sums

            # Step 4: the archive's SHA-256 against its line in SHA256SUMS.
            # Get-FileHash and Expand-Archive are module functions, which do
            # not see this scope's $ErrorActionPreference, hence -ErrorAction.
            $expected = $null
            foreach ($line in [IO.File]::ReadAllLines($sums)) {
                $fields = @($line.Trim() -split '\s+')
                if ($fields.Count -eq 2 -and $fields[1].TrimStart('*') -ceq $archive) {
                    $expected = $fields[0].ToLowerInvariant()
                    break
                }
            }
            if ($null -eq $expected) {
                throw "SHA256SUMS has no line for $archive, so it cannot be checked; nothing was installed"
            }
            $actual = (Get-FileHash -LiteralPath $zip -Algorithm SHA256 -ErrorAction Stop).Hash.ToLowerInvariant()
            if ($actual -cne $expected) {
                throw "$archive does not match its SHA-256 in SHA256SUMS (expected $expected, got $actual); nothing was installed"
            }
            Say "checked $archive against SHA256SUMS"

            $unpacked = Join-Path $tmp 'unpacked'
            Expand-Archive -LiteralPath $zip -DestinationPath $unpacked -ErrorAction Stop
            $source = Join-Path (Join-Path $unpacked $name) 'nova.exe'
            if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
                throw "$archive holds no $name\nova.exe"
            }

            # Step 5: written beside the old nova, run there (step 6), and
            # only then moved in, so a failed install, or a nova that cannot
            # run on this machine, leaves the old one. The staged name ends
            # in .exe: PowerShell runs no other file itself.
            New-Item -ItemType Directory -Path $bin -Force | Out-Null
            $bin = (Get-Item -LiteralPath $bin).FullName
            $exe = Join-Path $bin 'nova.exe'
            $old = Join-Path $bin 'nova.exe.old'
            $staged = Join-Path $bin ('nova-install-' + [Guid]::NewGuid().ToString('N') + '.exe')
            Copy-Item -LiteralPath $source -Destination $staged

            # Step 6: the new nova says what it is, before it replaces
            # anything.
            $ran = $false
            $why = "'nova version' failed"
            try {
                $printed = @(& $staged version)
                $ran = $LASTEXITCODE -eq 0
            } catch {
                $why = $_.Exception.GetBaseException().Message
            }
            if (-not $ran) {
                throw "the new nova does not run on this machine ($why), so nothing was replaced; install it with: cargo install nova-cli --locked"
            }

            # A nova.exe that is running, such as the language server VS
            # Code started, can be renamed but not replaced or deleted. So
            # the old one becomes nova.exe.old, the new one moves in, and the
            # old one is removed if it can be, else by the next run; if the
            # new one cannot move in, the old one is renamed back.
            if (Test-Path -LiteralPath $old) {
                Remove-Item -LiteralPath $old -Force -ErrorAction SilentlyContinue
                if (Test-Path -LiteralPath $old) {
                    throw "$old, an older nova, is still running, so nothing was replaced; close the programs that use nova, VS Code among them, and run this again"
                }
            }
            $hadOld = Test-Path -LiteralPath $exe
            if ($hadOld) {
                try {
                    Move-Item -LiteralPath $exe -Destination $old
                } catch {
                    throw "could not rename $exe to nova.exe.old ($($_.Exception.Message)), so nothing was replaced"
                }
            }
            try {
                Move-Item -LiteralPath $staged -Destination $exe
            } catch {
                $why = $_.Exception.Message
                if ($hadOld) {
                    Move-Item -LiteralPath $old -Destination $exe -ErrorAction SilentlyContinue
                    if (-not (Test-Path -LiteralPath $exe)) {
                        throw "could not move the new nova into $bin ($why), nor the old one back: rename $old to nova.exe"
                    }
                }
                throw "could not move the new nova into $bin ($why), so nothing was replaced"
            }
            $staged = $null
            if ($hadOld) {
                Remove-Item -LiteralPath $old -Force -ErrorAction SilentlyContinue
            }
            Say "installed $exe"
            foreach ($printedLine in $printed) { Write-Host $printedLine }
        } finally {
            if ($staged -and (Test-Path -LiteralPath $staged)) {
                Remove-Item -LiteralPath $staged -Force -ErrorAction SilentlyContinue
            }
            Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
        }

        if ($env:NOVA_INSTALL_NO_PATH -eq '1') {
            Say "NOVA_INSTALL_NO_PATH is 1, so PATH was left alone; nova is in $bin"
        } else {
            Add-ToUserPath $bin
        }
    }

    try {
        Install-Nova
    } catch {
        $message = "nova: error: $($_.Exception.Message)"
        if (Get-Variable -Name PSCommandPath -ValueOnly -ErrorAction SilentlyContinue) {
            [Console]::Error.WriteLine($message)
            exit 1
        }
        throw $message
    }
}

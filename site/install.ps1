# Installs Jokbet on Windows, or updates it in place:
#
#   irm https://jokbet.jokerben.top/install.ps1 | iex
#
# or, from cmd:
#
#   powershell -NoProfile -Command "irm https://jokbet.jokerben.top/install.ps1 | iex"
#
# What a browser downloads carries the mark of the web, and SmartScreen stops an
# unsigned build that carries it. PowerShell's own download does not mark the
# file, so neither the installer nor the app it installs meets that prompt. What
# it installs is the newest release's Windows installer — the file the download
# page offers — checked against the SHA-256 the site lists for it, run silently
# the way the app's own updater runs it, and then opened.
#
#   JOKBET_OPEN=0  leave it closed afterwards
#
# Everything is inside Install-Jokbet, called on the last line, so a download
# cut short defines the function and runs nothing.

$ErrorActionPreference = 'Stop'

# Chinese or English, whichever the system speaks.
$zh = $false
foreach ($name in @('LC_ALL', 'LC_MESSAGES', 'LANG')) {
  if ((Get-Item -Path "env:$name" -ErrorAction SilentlyContinue) -and (Get-Item "env:$name").Value -like 'zh*') { $zh = $true }
}
if ((Get-UICulture).TwoLetterISOLanguageName -eq 'zh') { $zh = $true }

function Say {
  param([string]$Zh, [string]$En)
  if ($zh) { Write-Host $Zh } else { Write-Host $En }
}

function Install-Jokbet {
  # Windows PowerShell 5.1 still offers TLS 1.0 by default on older builds.
  [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
  $site = 'https://jokbet.jokerben.top'
  $tmp = Join-Path ([IO.Path]::GetTempPath()) ('jokbet-' + [Guid]::NewGuid().ToString('N'))
  New-Item -ItemType Directory -Path $tmp | Out-Null
  try {
    try {
      $manifest = Invoke-RestMethod "$site/api/latest.json" -UseBasicParsing
    } catch {
      throw (Say '读不到最新版本的信息，请稍后再试，或到 https://jokbet.jokerben.top/download 下载。' `
        'Could not read the newest release. Try again later, or download it from https://jokbet.jokerben.top/download.')
    }
    $file = $manifest.files.'windows-x64'
    if (-not $manifest.tag -or -not $file) {
      throw (Say '现在还没有可以安装的版本。' 'There is no release to install yet.')
    }
    $name = $file.name
    $exe = Join-Path $tmp $name

    Say "正在下载 Jokbet $($manifest.version)……" "Downloading Jokbet $($manifest.version)…"
    try {
      Invoke-WebRequest "$site/dl/$($manifest.tag)/$name" -OutFile $exe -UseBasicParsing
    } catch {
      # The mirror is unreachable: the same file is on GitHub.
      Invoke-WebRequest "$($manifest.downloadBase)/$name" -OutFile $exe -UseBasicParsing
    }

    if ($file.sha256) {
      $got = (Get-FileHash -Algorithm SHA256 -Path $exe).Hash.ToLower()
      if ($got -ne $file.sha256.ToLower()) {
        throw (Say '下载的文件和发布的校验值对不上，已停止安装。' `
          'The download does not match its published SHA-256; nothing was installed.')
      }
    } else {
      Say '（这个版本没有公布校验值，跳过校验。）' '(This release lists no SHA-256; not checked.)'
    }

    # /S is what the app's own updater passes: the installer closes a running
    # copy itself, the same way, so today's counts are not lost to this.
    $install = Start-Process -FilePath $exe -ArgumentList '/S' -PassThru -Wait
    if ($install.ExitCode -ne 0) {
      throw (Say "安装程序返回了 $($install.ExitCode)，安装没有完成。" "The installer exited with $($install.ExitCode); nothing was installed.")
    }

    Say "Jokbet $($manifest.version) 已安装。" "Installed Jokbet $($manifest.version)."
    if ($env:JOKBET_OPEN -ne '0') {
      $app = @(
        (Join-Path $env:LOCALAPPDATA 'Jokbet\Jokbet.exe'),
        (Join-Path $env:ProgramFiles 'Jokbet\Jokbet.exe')
      ) | Where-Object { Test-Path $_ } | Select-Object -First 1
      if ($app) {
        # Nothing to grant on the way in: Windows asks for no permission to
        # watch the keyboard, so there is no note to leave here.
        Start-Process $app
      } else {
        Say '安装完成，但没有找到 Jokbet.exe，请从开始菜单打开它。' `
          'Installed, but Jokbet.exe was not where it was expected; open it from the Start menu.'
      }
    }
  } finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
  }
}

try {
  Install-Jokbet
} catch {
  Write-Host "jokbet: $($_.Exception.Message)" -ForegroundColor Red
}

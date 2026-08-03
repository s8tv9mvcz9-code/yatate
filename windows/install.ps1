# 矢立（文語 IME）— Windows への導入・取り外し
#
#   導入:   管理者の PowerShell で  .\install.ps1
#   取外し: 管理者の PowerShell で  .\install.ps1 -Uninstall
#
# 何をするか:
#   ① 機械の種別（ARM64 / x64）と DLL の種別が合つてゐるかを確かめる
#   ② DLL を恒久の場所へ写す（**レジストリがその道を指すので、後で消してはいけない**）
#   ③ regsvr32 で登録する（DllRegisterServer が COM と TSF の両方へ名乗る）
#
# 註: 未署名の DLL である。手元の検証はこれで通るが、配布には署名を用意すること
#     （OSS は SignPath Foundation の無償証明書といふ道がある）。
#
# ⚠ **このファイルは UTF-8 BOM 付きで保存すること。**
#   Windows PowerShell 5.1 は BOM 無しの .ps1 をシステム ANSI（日本語環境では CP932）
#   として読む。すると日本語のコメントと文字列が化け、化けた二バイト文字が直後の `"` を
#   食ひ潰して**構文エラーで起動すらしない**（`式またはステートメントのトークン
#   'ARM64" }' を使用できません` の類）。DLL は無傷なのに導入だけが不能になり、
#   症状から原因を辿るのは難しい。pwsh 7 は BOM 無しでも正しく読むが、
#   配る相手の既定は 5.1 なので、当てにしてはいけない。

[CmdletBinding()]
param(
    [switch]$Uninstall,
    # 既定は Program Files。書き込めない場合はここを変へる。
    [string]$InstallDir = "$env:ProgramFiles\Yatate"
)

$ErrorActionPreference = "Stop"
$DllName = "yatate_windows.dll"

function Assert-Admin {
    $id = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($id)
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw "管理者として実行してください（登録は HKEY_CLASSES_ROOT へ書くため）。"
    }
}

# 機械と DLL の種別が食ひ違ふと、登録は通るのに**どのアプリでも動かない**。
# ARM64 機のネイティブ処理（メモ帳・Edge 等）に x64 の DLL は読み込まれないからである。
# 症状から原因を辿るのは難しいので、入れる前に確かめる。
function Assert-Architecture([string]$dllPath) {
    $machine = $env:PROCESSOR_ARCHITECTURE   # ARM64 / AMD64
    $bytes = [System.IO.File]::ReadAllBytes($dllPath)
    $peOffset = [BitConverter]::ToInt32($bytes, 0x3C)
    $machineType = [BitConverter]::ToUInt16($bytes, $peOffset + 4)
    $dllArch = switch ($machineType) {
        0xAA64 { "ARM64" }
        0x8664 { "AMD64" }
        0x014C { "x86" }
        default { "不明($('0x{0:X}' -f $machineType))" }
    }
    Write-Host "  この機械: $machine ／ この DLL: $dllArch"
    if ($machine -eq "ARM64" -and $dllArch -ne "ARM64") {
        throw "この機械は ARM64 ですが、DLL は $dllArch です。arm64 版を使つてください（ARM64 のアプリは x64 の IME を読み込めません）。"
    }
    if ($machine -eq "AMD64" -and $dllArch -ne "AMD64") {
        throw "この機械は x64 ですが、DLL は $dllArch です。x64 版を使つてください。"
    }
}

function Install-Yatate {
    Assert-Admin
    $src = Join-Path $PSScriptRoot $DllName
    if (-not (Test-Path $src)) { throw "$DllName が見つかりません（このスクリプトと同じ場所に置いてください）。" }

    Assert-Architecture $src

    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    $dest = Join-Path $InstallDir $DllName

    # 既に登録されてゐると DLL が掴まれてゐて上書きできない。先に外す。
    if (Test-Path $dest) {
        Write-Host "  既存の登録を外します…"
        & regsvr32.exe /u /s $dest 2>$null
        Start-Sleep -Milliseconds 500
    }

    Copy-Item $src $dest -Force
    Write-Host "  写しました: $dest"

    Write-Host "  登録します…"
    $p = Start-Process regsvr32.exe -ArgumentList "/s", "`"$dest`"" -Wait -PassThru -Verb RunAs
    if ($p.ExitCode -ne 0) { throw "regsvr32 が失敗しました（終了コード $($p.ExitCode)）。" }

    Write-Host ""
    Write-Host "導入しました。" -ForegroundColor Green
    Write-Host "  次にすること:"
    Write-Host "   1. 設定 → 時刻と言語 → 言語と地域 → 日本語 → 「…」→ 言語のオプション"
    Write-Host "      → キーボード に「矢立（文語 IME）」が居るか確かめる"
    Write-Host "   2. 居なければ「キーボードの追加」から選ぶ"
    Write-Host "   3. Win+Space で矢立へ切り替へ、メモ帳で試す"
    Write-Host ""
    Write-Host "  原器の打ち方（JIS 鍵盤）:"
    Write-Host "    0=あ 9=い 8=う 7=え 6=お ／ p=か o=き i=く u=け y=こ"
    Write-Host "    :=さ ;=し l=す k=せ j=そ ／ 5=た 4=ち 3=つ 2=て 1=と"
    Write-Host "    t=な r=に e=ぬ w=ね q=の ／ g=は f=ひ d=ふ s=へ a=ほ"
    Write-Host "    ^=前置シフト（^^ で「ん」）／ b=濁点 v=半濁点（後置）"
    Write-Host "    例: u d g ^y o 2 ^^ o t ^4  →  けふはよきてんきなり"
    Write-Host "    Enter か Space で確定（**確定時に新字体は旧字体へ機械で直る**）／ Esc で取消"
}

function Uninstall-Yatate {
    Assert-Admin
    $dest = Join-Path $InstallDir $DllName
    if (-not (Test-Path $dest)) {
        Write-Host "  入つてゐません: $dest"
        return
    }
    & regsvr32.exe /u /s $dest
    Start-Sleep -Milliseconds 500
    Remove-Item $dest -Force -ErrorAction SilentlyContinue
    Write-Host "取り外しました。" -ForegroundColor Green
    Write-Host "  （DLL が掴まれてゐて消せない場合は、一度サインアウトしてから再実行してください）"
}

if ($Uninstall) { Uninstall-Yatate } else { Install-Yatate }

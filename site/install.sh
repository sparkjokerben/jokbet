#!/bin/sh
# Installs Jokbet on macOS, or updates it in place:
#
#   curl -fsSL https://jokbet.jokerben.top/install.sh | sh
#
# A browser marks what it downloads as quarantined, and Gatekeeper stops a
# quarantined build Apple has not notarized: a disk image when it is opened,
# and the app inside it once more. curl marks nothing, so an app installed this
# way opens straight away. What it installs is the newest release's disk image
# for this Mac — the file the download page offers — checked against the
# SHA-256 the site lists for it.
#
#   JOKBET_DIR     where Jokbet.app goes: /Applications, or ~/Applications
#                  when /Applications cannot be written to
#   JOKBET_OPEN=0  leave it closed afterwards
#
# Everything is inside main(), called on the last line, so a download cut short
# runs nothing at all.

set -eu

SITE=https://jokbet.jokerben.top
GITHUB=https://github.com/sparkjokerben/jokbet/releases/download

# Chinese or English, whichever the terminal speaks.
case "${LC_ALL:-${LC_MESSAGES:-${LANG:-}}}" in
  zh*) ZH=1 ;;
  *) ZH= ;;
esac

say() {
  if [ -n "${ZH}" ]; then printf '%s\n' "$1"; else printf '%s\n' "$2"; fi
}

fail() {
  if [ -n "${ZH}" ]; then printf 'jokbet: %s\n' "$1" >&2; else printf 'jokbet: %s\n' "$2" >&2; fi
  exit 1
}

# The version, tag, file name and SHA-256 of one installer in the manifest,
# one per line. JavaScript for Automation reads the JSON: every Mac has it.
installer() {
  osascript -l JavaScript -e '
    function run(argv) {
      var m = JSON.parse(argv[0]);
      var f = m.files && m.files[argv[1]];
      return m.tag && f ? [m.version, m.tag, f.name, f.sha256 || ""].join("\n") : "";
    }' "$1" "$2" </dev/null
}

# Downloads $1 to $2, with the progress bar on the terminal.
fetch() {
  curl -fL --retry 2 --connect-timeout 15 --progress-bar -o "$2" "$1"
}

# True while the app at $1 is running.
running() {
  pgrep -f "^$1/Contents/MacOS/" >/dev/null 2>&1
}

main() {
  [ "$(uname -s)" = Darwin ] ||
    fail "这个脚本只用于 macOS。其他系统请到 ${SITE}/download 下载。" \
      "This installer is for macOS. For other systems, see ${SITE}/download."
  major="$(sw_vers -productVersion | cut -d. -f1)"
  [ "${major}" -ge 11 ] ||
    fail "Jokbet 需要 macOS 11 或更新的版本。" "Jokbet needs macOS 11 or later."

  # Apple silicon, even from a shell that runs under Rosetta.
  if [ "$(sysctl -n hw.optional.arm64 2>/dev/null || echo 0)" = 1 ]; then
    platform=macos-aarch64
  else
    platform=macos-x64
  fi

  tmp="$(mktemp -d "${TMPDIR:-/tmp}/jokbet.XXXXXX")"
  mount=""
  staged=""
  trap 'cleanup' EXIT
  trap 'exit 130' INT TERM

  curl -fsSL --retry 2 --connect-timeout 15 -o "${tmp}/latest.json" "${SITE}/api/latest.json" ||
    fail "读不到最新版本的信息，请稍后再试，或到 ${SITE}/download 下载。" \
      "Could not read the newest release. Try again later, or download it from ${SITE}/download."
  info="$(installer "$(cat "${tmp}/latest.json")" "${platform}")" ||
    fail "看不懂最新版本的信息，请到 ${SITE}/download 下载。" \
      "Could not make sense of the newest release. Download it from ${SITE}/download."
  [ -n "${info}" ] ||
    fail "现在还没有可以安装的版本。" "There is no release to install yet."
  # The last line is empty when the release lists no SHA-256, and read fails
  # at the end of its input; that is not an error here.
  sha256=""
  {
    read -r version
    read -r tag
    read -r name
    read -r sha256 || true
  } <<EOF
${info}
EOF

  say "正在下载 Jokbet ${version}……" "Downloading Jokbet ${version}…"
  fetch "${SITE}/dl/${tag}/${name}" "${tmp}/${name}" || fetch "${GITHUB}/${tag}/${name}" "${tmp}/${name}" ||
    fail "下载失败，请检查网络后重试。" "The download failed. Check the connection and try again."

  if [ -n "${sha256}" ]; then
    [ "$(shasum -a 256 "${tmp}/${name}" | cut -d' ' -f1)" = "${sha256}" ] ||
      fail "下载的文件和发布的校验值对不上，已停止安装。" \
        "The download does not match its published SHA-256; nothing was installed."
  else
    say "（这个版本没有公布校验值，跳过校验。）" "(This release lists no SHA-256; not checked.)"
  fi

  mount="${tmp}/volume"
  mkdir "${mount}"
  # Its complaints (newer systems call it deprecated) only matter if it fails.
  if ! hdiutil attach -nobrowse -readonly -noautoopen -mountpoint "${mount}" "${tmp}/${name}" \
    >/dev/null 2>"${tmp}/attach.log" </dev/null; then
    cat "${tmp}/attach.log" >&2
    fail "打不开下载的磁盘映像。" "Could not open the downloaded disk image."
  fi
  [ -d "${mount}/Jokbet.app" ] ||
    fail "磁盘映像里没有 Jokbet.app。" "The disk image holds no Jokbet.app."

  dir="${JOKBET_DIR:-/Applications}"
  if [ -z "${JOKBET_DIR:-}" ] && [ ! -w /Applications ]; then
    dir="${HOME}/Applications"
  fi
  mkdir -p "${dir}"
  app="${dir}/Jokbet.app"

  # Staged beside the old copy, so the swap below is two renames and a copy
  # that fails half-way leaves the old one where it was.
  staged="${dir}/.Jokbet.app.installing"
  rm -rf "${staged}"
  ditto "${mount}/Jokbet.app" "${staged}"

  if running "${app}"; then
    say "正在关闭正在运行的 Jokbet……" "Quitting the running Jokbet…"
    # Asked like the Quit item does, so it saves today's counts on the way out.
    osascript -e 'on run argv' -e 'tell application (item 1 of argv) to quit' -e 'end run' "${app}" \
      </dev/null >/dev/null 2>&1 || true
    i=0
    while running "${app}" && [ "${i}" -lt 20 ]; do
      sleep 0.5
      i=$((i + 1))
    done
    ! running "${app}" ||
      fail "Jokbet 没有退出，请先从托盘菜单退出它再重试。" \
        "Jokbet did not quit. Quit it from its tray menu, then try again."
  fi
  rm -rf "${app}"
  mv "${staged}" "${app}"

  say "Jokbet ${version} 已安装到 ${app}" "Installed Jokbet ${version} in ${app}"
  if [ "${JOKBET_OPEN:-1}" != 0 ]; then
    open "${app}"
    say "第一次打开时，按提示授予「输入监控」权限，它才能计数。" \
      "The first time, allow Input Monitoring when asked, so it can count."
  fi
}

cleanup() {
  status=$?
  [ -z "${staged}" ] || rm -rf "${staged}"
  if [ -n "${mount}" ]; then
    hdiutil detach -quiet "${mount}" </dev/null 2>/dev/null || hdiutil detach -quiet -force "${mount}" </dev/null 2>/dev/null || true
  fi
  rm -rf "${tmp}"
  exit "${status}"
}

main "$@"

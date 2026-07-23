//! Shell integration installed entirely inside pH7Console's application data.
//!
//! No user startup file is edited. Zsh is launched through a private `ZDOTDIR`
//! which delegates to the user's real startup files, Bash uses a private
//! `--rcfile`, and Fish sources the integration with `--init-command` after its
//! normal configuration has loaded.
//!
//! The scripts emit the FinalTerm/Warp-compatible OSC 133 lifecycle markers,
//! OSC 7 working-directory updates, and one pH7-specific, percent-encoded event:
//!
//! ```text
//! OSC 1337;pH7;event=command_end;status=<integer>;truncated=<0|1>;command=<value> BEL
//! ```
//!
//! `command` is capped, percent-encoded, and replaced with a redaction marker
//! when it looks likely to contain a credential. This protocol is local to the
//! PTY; this module performs no networking or persistence of command history.

use portable_pty::CommandBuilder;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const INTEGRATION_VERSION: &str = "v1";
const INTEGRATION_DIRECTORY: &str = "shell-integration";

pub const MAX_RECORDED_COMMAND_BYTES: usize = 8 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShellKind {
    Zsh,
    Bash,
    Fish,
    Unsupported,
}

impl ShellKind {
    pub fn detect(shell: impl AsRef<Path>) -> Self {
        let executable = shell
            .as_ref()
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .trim_start_matches('-')
            .trim_end_matches(".exe")
            .to_ascii_lowercase();

        match executable.as_str() {
            "zsh" => Self::Zsh,
            "bash" => Self::Bash,
            "fish" => Self::Fish,
            _ => Self::Unsupported,
        }
    }
}

/// Arguments and environment overrides required for one shell process.
///
/// Call [`ShellLaunchConfig::apply_to`] after constructing a portable-pty
/// `CommandBuilder`, and before spawning the command. Unsupported shells are
/// deliberately left unchanged so the caller can use its normal fallback.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShellLaunchConfig {
    pub shell_kind: ShellKind,
    pub args: Vec<OsString>,
    pub environment: Vec<(OsString, OsString)>,
    pub integration_enabled: bool,
}

impl ShellLaunchConfig {
    pub fn apply_to(&self, command: &mut CommandBuilder) {
        command.args(&self.args);
        for (key, value) in &self.environment {
            command.env(key, value);
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShellIntegration {
    root: PathBuf,
}

impl ShellIntegration {
    /// Installs versioned, read-only-at-runtime integration files below the
    /// supplied application-data directory. Existing matching files are reused.
    pub fn install(app_data_directory: impl AsRef<Path>) -> io::Result<Self> {
        let root = app_data_directory
            .as_ref()
            .join(INTEGRATION_DIRECTORY)
            .join(INTEGRATION_VERSION);
        let zsh_directory = root.join("zsh");

        create_private_directory(&root)?;
        create_private_directory(&zsh_directory)?;

        write_private_file(&root.join("ph7-integration.zsh"), ZSH_INTEGRATION)?;
        write_private_file(&root.join("ph7-integration.bash"), BASH_INTEGRATION)?;
        write_private_file(&root.join("ph7-integration.fish"), FISH_INTEGRATION)?;
        write_private_file(&root.join("ph7-bashrc"), BASH_WRAPPER)?;

        write_private_file(&zsh_directory.join(".zshenv"), ZSHENV_WRAPPER)?;
        write_private_file(&zsh_directory.join(".zprofile"), ZPROFILE_WRAPPER)?;
        write_private_file(&zsh_directory.join(".zshrc"), ZSHRC_WRAPPER)?;
        write_private_file(&zsh_directory.join(".zlogin"), ZLOGIN_WRAPPER)?;
        write_private_file(&zsh_directory.join(".zlogout"), ZLOGOUT_WRAPPER)?;

        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Produces a launch configuration using the same environment map already
    /// associated with the terminal session. Values are passed to the process,
    /// never interpolated into a shell command or written into generated files.
    pub fn launch_config(
        &self,
        shell: impl AsRef<Path>,
        inherited_environment: &HashMap<String, String>,
    ) -> ShellLaunchConfig {
        let shell_kind = ShellKind::detect(shell);
        let common_environment = |integration: &str| {
            vec![
                (
                    OsString::from("PH7_SHELL_INTEGRATION"),
                    self.root.join(integration).into_os_string(),
                ),
                (
                    OsString::from("PH7_SHELL_INTEGRATION_VERSION"),
                    OsString::from(INTEGRATION_VERSION),
                ),
            ]
        };

        match shell_kind {
            ShellKind::Zsh => {
                let wrapper_zdotdir = self.root.join("zsh").into_os_string();
                let user_zdotdir = inherited_environment
                    .get("ZDOTDIR")
                    .or_else(|| inherited_environment.get("HOME"))
                    .map(OsString::from)
                    .unwrap_or_default();
                let mut environment = common_environment("ph7-integration.zsh");
                environment.extend([
                    (OsString::from("PH7_USER_ZDOTDIR"), user_zdotdir),
                    (
                        OsString::from("PH7_WRAPPER_ZDOTDIR"),
                        wrapper_zdotdir.clone(),
                    ),
                    (OsString::from("ZDOTDIR"), wrapper_zdotdir),
                ]);

                ShellLaunchConfig {
                    shell_kind,
                    args: vec![OsString::from("-l")],
                    environment,
                    integration_enabled: true,
                }
            }
            ShellKind::Bash => ShellLaunchConfig {
                shell_kind,
                args: vec![
                    OsString::from("--rcfile"),
                    self.root.join("ph7-bashrc").into_os_string(),
                    OsString::from("-i"),
                ],
                environment: common_environment("ph7-integration.bash"),
                integration_enabled: true,
            },
            ShellKind::Fish => ShellLaunchConfig {
                shell_kind,
                args: vec![
                    OsString::from("--login"),
                    OsString::from("--init-command"),
                    OsString::from("source \"$PH7_SHELL_INTEGRATION\""),
                ],
                environment: common_environment("ph7-integration.fish"),
                integration_enabled: true,
            },
            ShellKind::Unsupported => ShellLaunchConfig {
                shell_kind,
                args: Vec::new(),
                environment: Vec::new(),
                integration_enabled: false,
            },
        }
    }

    /// Convenience for callers that use portable-pty's inherited process
    /// environment rather than a precomputed terminal-session environment.
    pub fn launch_config_from_process(&self, shell: impl AsRef<Path>) -> ShellLaunchConfig {
        let environment = std::env::vars().collect::<HashMap<_, _>>();
        self.launch_config(shell, &environment)
    }
}

fn create_private_directory(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path)?;
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("refusing to install shell integration through symlink: {path:?}"),
        ));
    }
    set_private_directory_permissions(path)
}

fn write_private_file(path: &Path, contents: &str) -> io::Result<()> {
    if fs::read(path).ok().as_deref() == Some(contents.as_bytes()) {
        return set_private_file_permissions(path);
    }

    let parent = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "shell integration path has no parent directory",
        )
    })?;
    create_private_directory(parent)?;

    let mut temporary_path = None;
    for attempt in 0..32_u8 {
        let candidate = parent.join(format!(
            ".ph7-shell-integration-{}-{attempt}.tmp",
            std::process::id()
        ));
        match OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&candidate)
        {
            Ok(mut file) => {
                file.write_all(contents.as_bytes())?;
                file.sync_all()?;
                set_private_file_permissions(&candidate)?;
                temporary_path = Some(candidate);
                break;
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }

    let temporary_path = temporary_path.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not allocate a temporary shell integration file",
        )
    })?;

    #[cfg(windows)]
    if path.exists() {
        fs::remove_file(path)?;
    }

    if let Err(error) = fs::rename(&temporary_path, path) {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }
    set_private_file_permissions(path)
}

#[cfg(unix)]
fn set_private_directory_permissions(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
fn set_private_directory_permissions(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn set_private_file_permissions(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn set_private_file_permissions(_path: &Path) -> io::Result<()> {
    Ok(())
}

const ZSHENV_WRAPPER: &str = r#"# Generated by pH7Console. User files are sourced, never modified.
typeset -g PH7_USER_ZDOTDIR="${PH7_USER_ZDOTDIR:-${HOME:-}}"
if [[ -n "${PH7_USER_ZDOTDIR:-}" &&
      "${PH7_USER_ZDOTDIR}" != "${PH7_WRAPPER_ZDOTDIR:-}" &&
      -r "${PH7_USER_ZDOTDIR}/.zshenv" ]]; then
  builtin source "${PH7_USER_ZDOTDIR}/.zshenv"
fi
if [[ -n "${ZDOTDIR:-}" && "${ZDOTDIR}" != "${PH7_WRAPPER_ZDOTDIR:-}" ]]; then
  typeset -g PH7_USER_ZDOTDIR="${ZDOTDIR}"
fi
typeset -gx ZDOTDIR="${PH7_WRAPPER_ZDOTDIR}"
"#;

const ZPROFILE_WRAPPER: &str = r#"# Generated by pH7Console. User files are sourced, never modified.
if [[ -n "${PH7_USER_ZDOTDIR:-}" &&
      "${PH7_USER_ZDOTDIR}" != "${PH7_WRAPPER_ZDOTDIR:-}" &&
      -r "${PH7_USER_ZDOTDIR}/.zprofile" ]]; then
  builtin source "${PH7_USER_ZDOTDIR}/.zprofile"
fi
"#;

const ZSHRC_WRAPPER: &str = r#"# Generated by pH7Console. User files are sourced, never modified.
if [[ -n "${PH7_USER_ZDOTDIR:-}" &&
      "${PH7_USER_ZDOTDIR}" != "${PH7_WRAPPER_ZDOTDIR:-}" &&
      -r "${PH7_USER_ZDOTDIR}/.zshrc" ]]; then
  builtin source "${PH7_USER_ZDOTDIR}/.zshrc"
fi
if [[ -r "${PH7_SHELL_INTEGRATION:-}" ]]; then
  builtin source "${PH7_SHELL_INTEGRATION}"
fi
"#;

const ZLOGIN_WRAPPER: &str = r#"# Generated by pH7Console. User files are sourced, never modified.
if [[ -n "${PH7_USER_ZDOTDIR:-}" &&
      "${PH7_USER_ZDOTDIR}" != "${PH7_WRAPPER_ZDOTDIR:-}" &&
      -r "${PH7_USER_ZDOTDIR}/.zlogin" ]]; then
  builtin source "${PH7_USER_ZDOTDIR}/.zlogin"
fi
"#;

const ZLOGOUT_WRAPPER: &str = r#"# Generated by pH7Console. User files are sourced, never modified.
if [[ -n "${PH7_USER_ZDOTDIR:-}" &&
      "${PH7_USER_ZDOTDIR}" != "${PH7_WRAPPER_ZDOTDIR:-}" &&
      -r "${PH7_USER_ZDOTDIR}/.zlogout" ]]; then
  builtin source "${PH7_USER_ZDOTDIR}/.zlogout"
fi
"#;

const BASH_WRAPPER: &str = r#"# Generated by pH7Console. User files are sourced, never modified.
# `--rcfile` gives pH7 a reliable integration point. Reproduce login-style
# startup without changing HOME or any user-owned file.
if [[ -r /etc/profile ]]; then
  source /etc/profile
fi
if [[ -r "${HOME:-}/.bash_profile" ]]; then
  source "${HOME}/.bash_profile"
elif [[ -r "${HOME:-}/.bash_login" ]]; then
  source "${HOME}/.bash_login"
elif [[ -r "${HOME:-}/.profile" ]]; then
  source "${HOME}/.profile"
elif [[ -r "${HOME:-}/.bashrc" ]]; then
  source "${HOME}/.bashrc"
fi
if [[ -r "${PH7_SHELL_INTEGRATION:-}" ]]; then
  source "${PH7_SHELL_INTEGRATION}"
fi
"#;

const ZSH_INTEGRATION: &str = r#"# pH7Console shell integration v1 (zsh)
[[ -o interactive ]] || return 0
[[ -z "${__PH7_ZSH_INTEGRATION_ACTIVE:-}" ]] || return 0
typeset -g __PH7_ZSH_INTEGRATION_ACTIVE=1
typeset -g __ph7_command_active=0
typeset -g __ph7_last_command=''
typeset -g __ph7_last_status=0
typeset -g __ph7_zle_marker_installed=0
typeset -gr __ph7_prompt_marker=$'%{\e]133;B\a%}'

__ph7_privacy_filter() {
  emulate -L zsh
  local command="$1"
  local lowered="${(L)command}"
  if [[ "$lowered" == *password=* || "$lowered" == *passwd=* ||
        "$lowered" == *token=* || "$lowered" == *secret=* ||
        "$lowered" == *api_key=* || "$lowered" == *apikey=* ||
        "$lowered" == *authorization:* || "$lowered" == *--password* ||
        "$lowered" == *--token* || "$lowered" == *--secret* ||
        "$lowered" == *'security add-generic-password'* ||
        "$lowered" == *'gh auth login'* || "$lowered" == *'npm login'* ]]; then
    print -rn -- '[redacted: possible credential]'
  else
    print -rn -- "$command"
  fi
}

__ph7_percent_encode() {
  emulate -L zsh
  setopt localoptions noshwordsplit
  local LC_ALL=C
  local input="$1" keep_slash="${2:-0}" output='' byte hex
  integer index maximum=${3:-8192}
  (( ${#input} > maximum )) && input="${input[1,maximum]}"
  for (( index = 1; index <= ${#input}; index++ )); do
    byte="${input[index]}"
    if [[ "$byte" == [A-Za-z0-9._~] || "$byte" == '-' ||
          ( "$keep_slash" == 1 && "$byte" == '/' ) ]]; then
      output+="$byte"
    else
      printf -v hex '%02X' "'$byte"
      output+="%$hex"
    fi
  done
  print -rn -- "$output"
}

__ph7_finish_command() {
  local exit_status=$?
  typeset -g __ph7_last_status=$exit_status
  if (( __ph7_command_active )); then
    local filtered encoded truncated=0
    filtered="$(__ph7_privacy_filter "$__ph7_last_command")"
    (( ${#filtered} > 8192 )) && truncated=1
    encoded="$(__ph7_percent_encode "$filtered" 0 8192)"
    printf '\e]133;D;%d\a' "$exit_status"
    printf '\e]1337;pH7;event=command_end;status=%d;truncated=%d;command=%s\a' \
      "$exit_status" "$truncated" "$encoded"
  fi
  typeset -g __ph7_command_active=0
  return "$exit_status"
}

__ph7_prompt_start() {
  local preserved_status=$__ph7_last_status encoded_pwd
  encoded_pwd="$(__ph7_percent_encode "$PWD" 1 16384)"
  printf '\e]7;file://%s\a' "$encoded_pwd"
  printf '\e]133;A\a'
  if (( ! __ph7_zle_marker_installed )); then
    PS1="${PS1//$__ph7_prompt_marker/}${__ph7_prompt_marker}"
  fi
  return "$preserved_status"
}

__ph7_preexec() {
  typeset -g __ph7_last_command="$1"
  typeset -g __ph7_command_active=1
  printf '\e]133;C\a'
}

__ph7_zle_line_init() {
  printf '\e]133;B\a'
}

# Capture status before user precmd hooks can overwrite `$?`, then emit the new
# prompt marker after those hooks have finished updating the prompt.
precmd_functions=(${precmd_functions:#__ph7_finish_command})
precmd_functions=(__ph7_finish_command $precmd_functions)
precmd_functions=(${precmd_functions:#__ph7_prompt_start} __ph7_prompt_start)
preexec_functions=(${preexec_functions:#__ph7_preexec} __ph7_preexec)

autoload -Uz add-zle-hook-widget 2>/dev/null
if add-zle-hook-widget line-init __ph7_zle_line_init 2>/dev/null; then
  typeset -g __ph7_zle_marker_installed=1
fi
"#;

const BASH_INTEGRATION: &str = r#"# pH7Console shell integration v1 (bash 3.2+)
[[ "$-" == *i* ]] || return 0
[[ -z "${__PH7_BASH_INTEGRATION_ACTIVE:-}" ]] || return 0
__PH7_BASH_INTEGRATION_ACTIVE=1
__ph7_command_active=0
__ph7_ready_for_command=0
__ph7_internal=0
__ph7_last_command=''
__ph7_last_status=0
__ph7_prompt_marker='\[\e]133;B\a\]'

__ph7_privacy_filter() {
  local command="$1"
  case "$command" in
    *[Pp][Aa][Ss][Ss][Ww][Oo][Rr][Dd]=*|*[Pp][Aa][Ss][Ss][Ww][Dd]=*|\
    *[Tt][Oo][Kk][Ee][Nn]=*|*[Ss][Ee][Cc][Rr][Ee][Tt]=*|\
    *[Aa][Pp][Ii]_[Kk][Ee][Yy]=*|*[Aa][Pp][Ii][Kk][Ee][Yy]=*|\
    *[Aa][Uu][Tt][Hh][Oo][Rr][Ii][Zz][Aa][Tt][Ii][Oo][Nn]:*|\
    *--[Pp][Aa][Ss][Ss][Ww][Oo][Rr][Dd]*|*--[Tt][Oo][Kk][Ee][Nn]*|\
    *--[Ss][Ee][Cc][Rr][Ee][Tt]*|*security\ add-generic-password*|\
    *gh\ auth\ login*|*npm\ login*)
      printf '%s' '[redacted: possible credential]'
      ;;
    *) printf '%s' "$command" ;;
  esac
}

__ph7_percent_encode() {
  local LC_ALL=C input="$1" keep_slash="${2:-0}" maximum="${3:-8192}"
  local output='' byte hex index
  for (( index = 0; index < ${#input} && index < maximum; index++ )); do
    byte="${input:index:1}"
    case "$byte" in
      [A-Za-z0-9._~-]|-) output="${output}${byte}" ;;
      /) if [[ "$keep_slash" == 1 ]]; then
           output="${output}/"
         else
           output="${output}%2F"
         fi ;;
      *) printf -v hex '%02X' "'$byte"
         output="${output}%${hex}" ;;
    esac
  done
  printf '%s' "$output"
}

__ph7_finish_command() {
  local exit_status=$?
  __ph7_internal=1
  __ph7_ready_for_command=0
  __ph7_last_status=$exit_status
  if (( __ph7_command_active )); then
    local filtered encoded truncated=0
    filtered="$(__ph7_privacy_filter "$__ph7_last_command")"
    (( ${#filtered} > 8192 )) && truncated=1
    encoded="$(__ph7_percent_encode "$filtered" 0 8192)"
    printf '\e]133;D;%d\a' "$exit_status"
    printf '\e]1337;pH7;event=command_end;status=%d;truncated=%d;command=%s\a' \
      "$exit_status" "$truncated" "$encoded"
  fi
  __ph7_command_active=0
  __ph7_internal=0
  return "$exit_status"
}

__ph7_prompt_start() {
  local preserved_status=$__ph7_last_status encoded_pwd
  __ph7_internal=1
  encoded_pwd="$(__ph7_percent_encode "$PWD" 1 16384)"
  printf '\e]7;file://%s\a' "$encoded_pwd"
  printf '\e]133;A\a'
  PS1="${PS1//$__ph7_prompt_marker/}${__ph7_prompt_marker}"
  __ph7_ready_for_command=1
  __ph7_internal=0
  return "$preserved_status"
}

__ph7_bash_preexec() {
  local pending_command="${1:-$BASH_COMMAND}" preserved_status=$?
  (( __ph7_internal || ! __ph7_ready_for_command || __ph7_command_active )) && \
    return "$preserved_status"

  __ph7_internal=1
  __ph7_ready_for_command=0
  __ph7_command_active=1
  __ph7_last_command="$(builtin fc -ln -1 2>/dev/null)"
  __ph7_last_command="${__ph7_last_command#"${__ph7_last_command%%[![:space:]]*}"}"
  [[ -n "$__ph7_last_command" ]] || __ph7_last_command="$pending_command"
  printf '\e]133;C\a'
  __ph7_internal=0
  return "$preserved_status"
}

# Keep every existing PROMPT_COMMAND entry. The first hook captures the true
# status; the last one emits the prompt after themes have updated PS1.
if [[ "$(declare -p PROMPT_COMMAND 2>/dev/null)" == "declare -a"* ]]; then
  PROMPT_COMMAND=(__ph7_finish_command "${PROMPT_COMMAND[@]}" __ph7_prompt_start)
else
  __ph7_original_prompt_command="${PROMPT_COMMAND:-}"
  PROMPT_COMMAND="__ph7_finish_command"
  [[ -z "$__ph7_original_prompt_command" ]] || \
    PROMPT_COMMAND="${PROMPT_COMMAND};${__ph7_original_prompt_command}"
  PROMPT_COMMAND="${PROMPT_COMMAND};__ph7_prompt_start"
fi

# bash-preexec already multiplexes DEBUG safely; use it when present. Otherwise
# preserve and chain the previous DEBUG trap before installing ours.
if [[ "$(declare -p preexec_functions 2>/dev/null)" == "declare -a"* ]]; then
  preexec_functions+=(__ph7_bash_preexec)
else
  __ph7_previous_debug_command=''
  __ph7_debug_definition="$(trap -p DEBUG)"
  if [[ "$__ph7_debug_definition" == "trap -- "*" DEBUG" ]]; then
    __ph7_debug_quoted="${__ph7_debug_definition#trap -- }"
    __ph7_debug_quoted="${__ph7_debug_quoted% DEBUG}"
    eval "__ph7_previous_debug_command=${__ph7_debug_quoted}"
  fi
  __ph7_debug_dispatch() {
    local pending_command="$1" preserved_status=$?
    if [[ -n "$__ph7_previous_debug_command" && "$__ph7_internal" == 0 ]]; then
      __ph7_internal=1
      eval "$__ph7_previous_debug_command"
      __ph7_internal=0
    fi
    __ph7_bash_preexec "$pending_command"
    return "$preserved_status"
  }
  trap '__ph7_debug_dispatch "$BASH_COMMAND"' DEBUG
fi
"#;

const FISH_INTEGRATION: &str = r#"# pH7Console shell integration v1 (fish 3.1+)
status is-interactive; or return 0
set -q __PH7_FISH_INTEGRATION_ACTIVE; and return 0
set -g __PH7_FISH_INTEGRATION_ACTIVE 1
set -g __ph7_command_active 0
set -g __ph7_last_command ''

function __ph7_privacy_filter --argument-names command
    if string match --quiet --regex --ignore-case \
        '(password=|passwd=|token=|secret=|api_key=|apikey=|authorization:|--password|--token|--secret|security add-generic-password|gh auth login|npm login)' \
        -- "$command"
        printf '%s' '[redacted: possible credential]'
    else
        printf '%s' "$command"
    end
end

function __ph7_percent_encode --argument-names value keep_slash maximum
    test -n "$maximum"; or set maximum 8192
    set value (string sub --length "$maximum" -- "$value")
    set -l encoded (string escape --style=url -- "$value")
    if test "$keep_slash" = 1
        set encoded (string replace --all '%2F' '/' -- "$encoded")
        set encoded (string replace --all '%2f' '/' -- "$encoded")
    end
    printf '%s' "$encoded"
end

function __ph7_preexec --on-event fish_preexec
    set -g __ph7_last_command (string join ' ' -- $argv)
    set -g __ph7_command_active 1
    printf '\e]133;C\a'
end

function __ph7_postexec --on-event fish_postexec
    set -l exit_status $status
    if test $__ph7_command_active -eq 1
        set -l filtered (__ph7_privacy_filter "$__ph7_last_command")
        set -l truncated 0
        test (string length -- "$filtered") -le 8192; or set truncated 1
        set -l encoded (__ph7_percent_encode "$filtered" 0 8192)
        printf '\e]133;D;%d\a' "$exit_status"
        printf '\e]1337;pH7;event=command_end;status=%d;truncated=%d;command=%s\a' \
            "$exit_status" "$truncated" "$encoded"
    end
    set -g __ph7_command_active 0
end

if functions --query fish_prompt
    functions --copy fish_prompt __ph7_original_fish_prompt
    function fish_prompt
        set -l encoded_pwd (__ph7_percent_encode "$PWD" 1 16384)
        printf '\e]7;file://%s\a' "$encoded_pwd"
        printf '\e]133;A\a'
        __ph7_original_fish_prompt
        printf '\e]133;B\a'
    end
end
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should follow the Unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "ph7-shell-integration-test-{}-{nonce}-{}",
                std::process::id(),
                NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).expect("test directory should be created");
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn environment() -> HashMap<String, String> {
        HashMap::from([
            ("HOME".to_string(), "/Users/example".to_string()),
            (
                "ZDOTDIR".to_string(),
                "/Users/example/.config/zsh".to_string(),
            ),
        ])
    }

    #[test]
    fn detects_supported_shells_by_executable_name() {
        assert_eq!(ShellKind::detect("/bin/zsh"), ShellKind::Zsh);
        assert_eq!(ShellKind::detect("/usr/local/bin/bash"), ShellKind::Bash);
        assert_eq!(ShellKind::detect("fish.exe"), ShellKind::Fish);
        assert_eq!(ShellKind::detect("/bin/nu"), ShellKind::Unsupported);
    }

    #[test]
    fn installs_every_wrapper_without_touching_a_home_directory() {
        let temporary = TestDirectory::new();
        let fake_home = temporary.0.join("home");
        fs::create_dir_all(&fake_home).expect("fake home should be created");

        let installation = ShellIntegration::install(temporary.0.join("app-data"))
            .expect("integration should install");

        for relative in [
            "ph7-integration.zsh",
            "ph7-integration.bash",
            "ph7-integration.fish",
            "ph7-bashrc",
            "zsh/.zshenv",
            "zsh/.zprofile",
            "zsh/.zshrc",
            "zsh/.zlogin",
            "zsh/.zlogout",
        ] {
            assert!(installation.root().join(relative).is_file(), "{relative}");
        }
        assert_eq!(
            fs::read_dir(fake_home)
                .expect("fake home should be readable")
                .count(),
            0
        );
    }

    #[test]
    fn installation_is_idempotent() {
        let temporary = TestDirectory::new();
        let first = ShellIntegration::install(&temporary.0).expect("first install should work");
        let second = ShellIntegration::install(&temporary.0).expect("second install should work");
        assert_eq!(first, second);
    }

    #[test]
    fn zsh_uses_private_zdotdir_and_preserves_the_user_value() {
        let temporary = TestDirectory::new();
        let installation = ShellIntegration::install(&temporary.0).expect("install should work");
        let config = installation.launch_config("/bin/zsh", &environment());

        assert!(config.integration_enabled);
        assert_eq!(config.shell_kind, ShellKind::Zsh);
        assert_eq!(config.args, [OsString::from("-l")]);
        assert!(config.environment.iter().any(|(key, value)| {
            key == "PH7_USER_ZDOTDIR" && value == "/Users/example/.config/zsh"
        }));
        assert!(config.environment.iter().any(|(key, value)| {
            key == "ZDOTDIR" && value == installation.root().join("zsh").as_os_str()
        }));
    }

    #[test]
    fn bash_and_fish_source_only_fixed_environment_variable_commands() {
        let temporary = TestDirectory::new();
        let installation = ShellIntegration::install(&temporary.0).expect("install should work");

        let bash = installation.launch_config("/bin/bash", &environment());
        assert_eq!(bash.args.first(), Some(&OsString::from("--rcfile")));
        assert!(bash.integration_enabled);

        let fish = installation.launch_config("/opt/homebrew/bin/fish", &environment());
        assert_eq!(fish.args[0], "--login");
        assert_eq!(fish.args[1], "--init-command");
        assert_eq!(fish.args[2], "source \"$PH7_SHELL_INTEGRATION\"");
        assert!(fish.integration_enabled);
    }

    #[test]
    fn unsupported_shell_is_not_modified() {
        let temporary = TestDirectory::new();
        let installation = ShellIntegration::install(&temporary.0).expect("install should work");
        let config = installation.launch_config("/bin/sh", &environment());

        assert_eq!(config.shell_kind, ShellKind::Unsupported);
        assert!(!config.integration_enabled);
        assert!(config.args.is_empty());
        assert!(config.environment.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn installation_permissions_are_private() {
        use std::os::unix::fs::PermissionsExt;

        let temporary = TestDirectory::new();
        let installation = ShellIntegration::install(&temporary.0).expect("install should work");
        let directory_mode = fs::metadata(installation.root())
            .expect("root metadata")
            .permissions()
            .mode()
            & 0o777;
        let file_mode = fs::metadata(installation.root().join("ph7-integration.zsh"))
            .expect("file metadata")
            .permissions()
            .mode()
            & 0o777;

        assert_eq!(directory_mode, 0o700);
        assert_eq!(file_mode, 0o600);
    }

    #[test]
    fn generated_integrations_include_required_protocol_and_redaction() {
        for script in [ZSH_INTEGRATION, BASH_INTEGRATION, FISH_INTEGRATION] {
            assert!(script.contains("]7;file://"));
            assert!(script.contains("]133;A"));
            assert!(script.contains("]133;B"));
            assert!(script.contains("]133;C"));
            assert!(script.contains("]133;D;"));
            assert!(script.contains("]1337;pH7;event=command_end"));
            assert!(script.contains("[redacted: possible credential]"));
        }
    }

    #[cfg(unix)]
    #[test]
    fn installed_scripts_pass_available_shell_syntax_checks() {
        let temporary = TestDirectory::new();
        let installation = ShellIntegration::install(&temporary.0).expect("install should work");

        assert_syntax_if_available(
            "zsh",
            &[
                installation.root().join("ph7-integration.zsh"),
                installation.root().join("zsh/.zshenv"),
                installation.root().join("zsh/.zshrc"),
            ],
        );
        assert_syntax_if_available(
            "bash",
            &[
                installation.root().join("ph7-integration.bash"),
                installation.root().join("ph7-bashrc"),
            ],
        );

        if Command::new("fish").arg("--version").output().is_ok() {
            let output = Command::new("fish")
                .arg("--no-execute")
                .arg(installation.root().join("ph7-integration.fish"))
                .output()
                .expect("fish syntax check should start");
            assert!(
                output.status.success(),
                "fish syntax check failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    #[cfg(unix)]
    fn assert_syntax_if_available(shell: &str, scripts: &[PathBuf]) {
        if Command::new(shell).arg("--version").output().is_err() {
            return;
        }

        for script in scripts {
            let output = Command::new(shell)
                .arg("-n")
                .arg(script)
                .output()
                .expect("shell syntax check should start");
            assert!(
                output.status.success(),
                "{shell} rejected {script:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}

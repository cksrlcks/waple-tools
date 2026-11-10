//! LocalTools CLI (테스트·자동화용)
//!
//! localtools-cli run <tool-id> <경로>... [--opts '<json>'] [--recursive] [--keep <suffix> | --out <dir>] [--dry-run]
//! localtools-cli undo <job_id>

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use localtools_core::batch::{self, JobSpec, OutputPolicy};

const USAGE: &str = "사용법:
  localtools-cli run <tool-id> <경로>... [--opts '<json>'] [--recursive] [--keep <suffix> | --out <dir>] [--dry-run]
  localtools-cli undo <job_id>";

#[derive(Debug)]
enum Command {
    Run {
        tool: String,
        spec: JobSpec,
        dry_run: bool,
    },
    Undo {
        job_id: String,
    },
}

fn parse(args: &[String]) -> Result<Command, String> {
    match args.first().map(String::as_str) {
        Some("undo") => match args {
            [_, id] => Ok(Command::Undo { job_id: id.clone() }),
            _ => Err("undo에는 job_id 하나가 필요합니다.".into()),
        },
        Some("run") => {
            let tool = args.get(1).ok_or("도구 id가 필요합니다.")?.clone();
            let mut spec = JobSpec {
                inputs: vec![],
                recursive: false,
                policy: OutputPolicy::default(),
                keep_if_larger: true,
                threads: 0,
                opts: serde_json::json!({}),
            };
            let mut dry_run = false;
            let mut it = args[2..].iter();
            while let Some(a) = it.next() {
                let mut value = || {
                    it.next()
                        .cloned()
                        .ok_or(format!("{a} 뒤에 값이 필요합니다."))
                };
                match a.as_str() {
                    "--recursive" => spec.recursive = true,
                    "--dry-run" => dry_run = true,
                    "--opts" => {
                        spec.opts = serde_json::from_str(&value()?)
                            .map_err(|e| format!("--opts JSON이 올바르지 않습니다: {e}"))?
                    }
                    "--keep" | "--out" => {
                        if spec.policy != OutputPolicy::default() {
                            return Err("--keep과 --out은 함께 쓸 수 없습니다.".into());
                        }
                        let v = value()?;
                        spec.policy = if a == "--keep" {
                            OutputPolicy::KeepOriginal { suffix: v }
                        } else {
                            OutputPolicy::OutputFolder { dir: v.into() }
                        };
                    }
                    flag if flag.starts_with("--") => {
                        return Err(format!("알 수 없는 옵션: {flag}"));
                    }
                    path => spec.inputs.push(PathBuf::from(path)),
                }
            }
            if spec.inputs.is_empty() {
                return Err("처리할 경로가 필요합니다.".into());
            }
            Ok(Command::Run {
                tool,
                spec,
                dry_run,
            })
        }
        _ => Err("명령은 run 또는 undo입니다.".into()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = match parse(&args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let log_dir = localtools_builtin::default_log_dir();
    let out = match cmd {
        Command::Undo { job_id } => batch::undo(&log_dir, &job_id)
            .map(|items| serde_json::to_string_pretty(&items))
            .map_err(|e| e.to_string()),
        Command::Run {
            tool,
            spec,
            dry_run,
        } => {
            let Some(tool) = localtools_builtin::find(&tool) else {
                eprintln!("도구를 찾을 수 없습니다: {tool}");
                return ExitCode::from(2);
            };
            if dry_run {
                Ok(serde_json::to_string_pretty(&batch::plan(&*tool, &spec)))
            } else {
                let job_id = localtools_builtin::new_job_id();
                let cancel = Arc::new(AtomicBool::new(false));
                batch::run(&*tool, &spec, &job_id, &log_dir, cancel, |p| {
                    eprintln!("{}/{}", p.done, p.total)
                })
                .map(|r| serde_json::to_string_pretty(&r))
                .map_err(|e| format!("작업 기록을 만들 수 없습니다: {e}"))
            }
        }
    };
    match out {
        Ok(Ok(json)) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Ok(Err(e)) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use localtools_core::batch::OutputPolicy;
    use std::path::PathBuf;

    fn args(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parse_run_with_flags() {
        let c = parse(&args(&[
            "run",
            "dev.copy",
            "a",
            "b",
            "--recursive",
            "--keep",
            "_x",
            "--opts",
            "{\"q\":1}",
        ]))
        .unwrap();
        let Command::Run {
            tool,
            spec,
            dry_run,
        } = c
        else {
            panic!("{c:?}")
        };
        assert_eq!(tool, "dev.copy");
        assert_eq!(spec.inputs, [PathBuf::from("a"), PathBuf::from("b")]);
        assert!(spec.recursive);
        assert_eq!(
            spec.policy,
            OutputPolicy::KeepOriginal {
                suffix: "_x".into()
            }
        );
        assert_eq!(spec.opts, serde_json::json!({"q": 1}));
        assert!(!dry_run);
    }

    #[test]
    fn parse_dry_run_and_out() {
        let c = parse(&args(&["run", "dev.copy", "a", "--out", "d", "--dry-run"])).unwrap();
        let Command::Run { spec, dry_run, .. } = c else {
            panic!()
        };
        assert_eq!(spec.policy, OutputPolicy::OutputFolder { dir: "d".into() });
        assert!(dry_run);
    }

    #[test]
    fn parse_rejects_keep_and_out_together() {
        assert!(
            parse(&args(&[
                "run", "dev.copy", "a", "--keep", "_x", "--out", "d"
            ]))
            .is_err()
        );
    }

    #[test]
    fn parse_rejects_missing_inputs_and_bad_json() {
        assert!(parse(&args(&["run", "dev.copy"])).is_err());
        assert!(parse(&args(&["run", "dev.copy", "a", "--opts", "{"])).is_err());
        assert!(parse(&args(&[])).is_err());
    }

    #[test]
    fn parse_undo() {
        let c = parse(&args(&["undo", "20261110-010203-004-1"])).unwrap();
        assert!(matches!(c, Command::Undo { job_id } if job_id == "20261110-010203-004-1"));
    }
}

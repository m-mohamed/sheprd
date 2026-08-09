use crate::config::FlokConfig;
use crate::error::{Result, SheprdError};
use crate::herdr::{self, FlokAgent};
use crate::project::Project;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const FACTORY_SCHEMA_VERSION: u32 = 1;
const MARKER_START_PREFIX: &str = "<<<SHEPRD_FACTORY_JSON_START:";
const MARKER_END_PREFIX: &str = "<<<SHEPRD_FACTORY_JSON_END:";
const MARKER_LIKE_PREFIX: &str = "<<<SHEPRD_FACTORY";
const LEGACY_MARKER_LIKE_PREFIX: &str = "<<<END_SHEPRD_FACTORY";
const MAX_CORRECTION_TURNS: usize = 2;
const MAX_CAPTURE_BYTES: usize = 8 * 1024;
const MAX_REVIEW_PATCH_BYTES: usize = 48 * 1024;
const MAX_AGENT_PROMPT_BYTES: usize = 60 * 1024;
const MAX_GIT_ADMIN_ENTRY_BYTES: usize = 8 * 1024;
const MAX_IGNORED_ENTRIES: usize = 100_000;
const MAX_IGNORED_ENUM_BYTES: usize = 16 * 1024 * 1024;
const MAX_IGNORED_TOTAL_FILE_BYTES: u64 = 64 * 1024 * 1024 * 1024;
const CHECK_POLL_INTERVAL: Duration = Duration::from_millis(25);
const CHECK_ENV_ALLOWLIST: &[&str] = &[
    "CARGO_HOME",
    "HOME",
    "LANG",
    "LC_ALL",
    "LOGNAME",
    "PATH",
    "RUSTC_WRAPPER",
    "RUSTUP_HOME",
    "TERM",
    "TMPDIR",
    "USER",
];

#[derive(Clone, Debug)]
pub struct FactoryRequest {
    pub task: String,
    pub allow_paths: Vec<String>,
    pub checks: Vec<String>,
    pub check_timeout_seconds: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PlanStep {
    pub id: String,
    pub objective: String,
    pub allow_paths: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PlanEnvelope {
    pub schema_version: u32,
    pub kind: String,
    pub nonce: String,
    pub summary: String,
    pub steps: Vec<PlanStep>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ImplementationEnvelope {
    pub schema_version: u32,
    pub kind: String,
    pub nonce: String,
    pub summary: String,
    #[serde(default)]
    pub claimed_changed_paths: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ReviewEnvelope {
    pub schema_version: u32,
    pub kind: String,
    pub nonce: String,
    pub reviewer: String,
    pub approved: bool,
    pub summary: String,
    #[serde(default)]
    pub findings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CheckResult {
    pub command: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub mutated_source: bool,
    pub duration_ms: u128,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct CheckAttempt {
    pub implementation_turn: usize,
    pub results: Vec<CheckResult>,
}

#[derive(Clone, Debug, Serialize)]
pub struct FactoryReceipt {
    pub schema_version: u32,
    pub run_id: String,
    pub project: String,
    pub task: String,
    pub allow_paths: Vec<String>,
    pub check_commands: Vec<String>,
    pub check_timeout_seconds: u64,
    pub workspace_id: Option<String>,
    pub plan: Option<PlanEnvelope>,
    pub implementations: Vec<ImplementationEnvelope>,
    pub check_attempts: Vec<CheckAttempt>,
    pub claude_review: Option<ReviewEnvelope>,
    pub opencode_review: Option<ReviewEnvelope>,
    pub changed_paths: Vec<String>,
    pub base_unchanged: bool,
    pub worker_head_unchanged: bool,
    pub accepted: bool,
    pub failure: Option<String>,
    pub trace_path: String,
    pub receipt_path: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GitSnapshot {
    head: String,
    status: String,
    ignored: IgnoredStateSnapshot,
}

struct RunContext<'a> {
    project: &'a Project,
    request: &'a FactoryRequest,
    allowed: Vec<PathBuf>,
    base_before: GitSnapshot,
    worker_path: Option<PathBuf>,
    worker_head: Option<String>,
}

#[derive(Clone, Debug)]
struct EnvelopeMarker {
    nonce: String,
    start: String,
    end: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SourceSnapshot {
    git_admin: GitAdminSnapshot,
    paths: Vec<String>,
    digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GitAdminSnapshot {
    metadata: FileMetadataSnapshot,
    content_digest: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct IgnoredStateSnapshot {
    entries: Vec<(String, FileMetadataSnapshot)>,
    total_file_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FileMetadataSnapshot {
    kind: FileKind,
    len: u64,
    mode: u32,
    device: u64,
    inode: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum FileKind {
    Regular,
    Directory,
    Symlink,
}

pub fn run(
    project: &Project,
    flok_config: &FlokConfig,
    request: FactoryRequest,
) -> Result<FactoryReceipt> {
    if request.task.trim().is_empty() {
        return Err(SheprdError::Message(
            "factory task must not be empty".into(),
        ));
    }
    if request.checks.is_empty() || request.checks.iter().any(|check| check.trim().is_empty()) {
        return Err(SheprdError::Message(
            "factory check commands must not be empty".into(),
        ));
    }
    if request.check_timeout_seconds == 0 {
        return Err(SheprdError::Message(
            "factory check timeout must be at least one second".into(),
        ));
    }
    let allowed = normalize_allow_paths(&request.allow_paths)?;
    let run_id = factory_run_id();
    let project_state = factory_state_root(project)?;
    let _lock = FactoryLock::acquire(&project_state, project)?;
    let run_dir = project_state.join(&run_id);
    create_private_dir(&run_dir)?;
    let trace_path = run_dir.join("trace.jsonl");
    let receipt_path = run_dir.join("receipt.json");
    let mut trace = TraceWriter::create(&trace_path)?;
    let base_before = git_snapshot(&project.path)?;
    let mut context = RunContext {
        project,
        request: &request,
        allowed,
        base_before,
        worker_path: None,
        worker_head: None,
    };
    let mut receipt = FactoryReceipt {
        schema_version: FACTORY_SCHEMA_VERSION,
        run_id: run_id.clone(),
        project: project.name.clone(),
        task: request.task.clone(),
        allow_paths: request.allow_paths.clone(),
        check_commands: request.checks.clone(),
        check_timeout_seconds: request.check_timeout_seconds,
        workspace_id: None,
        plan: None,
        implementations: Vec::new(),
        check_attempts: Vec::new(),
        claude_review: None,
        opencode_review: None,
        changed_paths: Vec::new(),
        base_unchanged: true,
        worker_head_unchanged: true,
        accepted: false,
        failure: None,
        trace_path: trace_path.display().to_string(),
        receipt_path: receipt_path.display().to_string(),
    };

    trace.append(
        "run",
        "started",
        json!({
            "run_id": run_id,
            "project": project.name,
            "allow_paths": request.allow_paths,
            "checks": request.checks,
        }),
    )?;

    if let Err(error) = execute_factory(&mut context, flok_config, &mut receipt, &mut trace) {
        receipt.failure = Some(error.to_string());
        trace.append("run", "failed", json!({ "error": error.to_string() }))?;
    }

    receipt.base_unchanged = git_snapshot(&project.path)? == context.base_before;
    if let (Some(worker_path), Some(worker_head)) = (
        context.worker_path.as_deref(),
        context.worker_head.as_deref(),
    ) {
        receipt.worker_head_unchanged = git_head(worker_path)? == worker_head;
        receipt.changed_paths = changed_paths(worker_path, worker_head)?;
    }
    if !receipt.base_unchanged && receipt.failure.is_none() {
        receipt.failure = Some("base checkout changed during the factory run".into());
    }
    if !receipt.worker_head_unchanged && receipt.failure.is_none() {
        receipt.failure = Some("Codex worker HEAD changed; factory runs must not commit".into());
    }
    match first_disallowed_path(&receipt.changed_paths, &context.allowed) {
        Ok(Some(path)) if receipt.failure.is_none() => {
            receipt.failure = Some(format!(
                "changed path is outside the declared allow paths: {path}"
            ));
        }
        Err(error) if receipt.failure.is_none() => receipt.failure = Some(error.to_string()),
        _ => {}
    }

    let checks_pass = receipt
        .check_attempts
        .last()
        .is_some_and(|attempt| attempt.results.iter().all(|check| check.success));
    let reviews_approve = receipt
        .claude_review
        .as_ref()
        .is_some_and(|review| review.approved)
        && receipt
            .opencode_review
            .as_ref()
            .is_some_and(|review| review.approved);
    if checks_pass && !reviews_approve && receipt.failure.is_none() {
        receipt.failure = Some("Claude and OpenCode must both approve acceptance".into());
    }
    receipt.accepted = receipt.failure.is_none()
        && receipt.base_unchanged
        && receipt.worker_head_unchanged
        && checks_pass
        && reviews_approve;

    trace.append(
        "run",
        if receipt.accepted {
            "accepted"
        } else {
            "rejected"
        },
        json!({
            "accepted": receipt.accepted,
            "base_unchanged": receipt.base_unchanged,
            "worker_head_unchanged": receipt.worker_head_unchanged,
            "changed_paths": receipt.changed_paths,
            "failure": receipt.failure,
        }),
    )?;
    write_json_atomic(&receipt_path, &receipt)?;
    Ok(receipt)
}

fn execute_factory(
    context: &mut RunContext<'_>,
    flok_config: &FlokConfig,
    receipt: &mut FactoryReceipt,
    trace: &mut TraceWriter,
) -> Result<()> {
    let flok = herdr::open_flok(context.project, flok_config)?;
    receipt.workspace_id = Some(flok.workspace_id.clone());
    trace.append(
        "flok",
        "ready",
        json!({ "workspace_id": flok.workspace_id, "action": flok.action, "healthy": flok.healthy }),
    )?;
    if !flok.healthy || flok.agents.len() != 4 {
        return Err(SheprdError::Message(format!(
            "factory requires a healthy four-agent Flok: {}",
            flok.warnings.join("; ")
        )));
    }

    let pi = agent(&flok.agents, "pi")?;
    let codex = agent(&flok.agents, "codex")?;
    let claude = agent(&flok.agents, "claude")?;
    let opencode = agent(&flok.agents, "opencode")?;
    let worker_path = PathBuf::from(&codex.cwd);
    let claude_before = git_snapshot(Path::new(&claude.cwd))?;
    let opencode_before = git_snapshot(Path::new(&opencode.cwd))?;
    let worker_head = git_head(&worker_path)?;
    if worker_head != context.base_before.head {
        return Err(SheprdError::Message(format!(
            "Codex worker HEAD is stale: worker={worker_head} base={}",
            context.base_before.head
        )));
    }
    if !changed_paths(&worker_path, &worker_head)?.is_empty() {
        return Err(SheprdError::Message(
            "Codex worker checkout must be clean before a factory run".into(),
        ));
    }
    context.worker_path = Some(worker_path.clone());
    context.worker_head = Some(worker_head.clone());
    verify_integrity(context)?;

    let plan_marker = EnvelopeMarker::fresh()?;
    let plan_prompt = plan_prompt(context.request, &plan_marker);
    let plan_result = run_agent_phase(pi, &plan_prompt, "plan", trace);
    verify_integrity(context)?;
    let plan_text = plan_result?;
    let plan: PlanEnvelope = parse_envelope(&plan_text, "plan", &plan_marker)?;
    validate_plan(&plan, &context.allowed)?;
    trace.append("plan", "parsed", serde_json::to_value(&plan)?)?;
    receipt.plan = Some(plan.clone());
    verify_integrity(context)?;

    let implementation_marker = EnvelopeMarker::fresh()?;
    let initial_prompt =
        implementation_prompt(context.request, &plan, 0, None, &implementation_marker)?;
    let implementation_ignored = ignored_state_snapshot(&worker_path)?;
    let implementation_result = run_agent_phase(codex, &initial_prompt, "implementation", trace);
    require_ignored_state_unchanged(
        &worker_path,
        &implementation_ignored,
        "Codex implementation",
    )?;
    let implementation_text = implementation_result?;
    let implementation: ImplementationEnvelope = parse_envelope(
        &implementation_text,
        "implementation",
        &implementation_marker,
    )?;
    verify_integrity(context)?;
    validate_implementation(&implementation, &context.allowed)?;
    trace.append(
        "implementation",
        "parsed",
        serde_json::to_value(&implementation)?,
    )?;
    receipt.implementations.push(implementation);

    for correction_turn in 0..=MAX_CORRECTION_TURNS {
        let results = run_checks(
            &worker_path,
            &worker_head,
            &context.request.checks,
            Duration::from_secs(context.request.check_timeout_seconds),
        )?;
        let post_check_ignored = ignored_state_snapshot(&worker_path)?;
        let all_pass = results.iter().all(|result| result.success);
        let check_mutated_source = results.iter().any(|result| result.mutated_source);
        let check_timed_out = results.iter().any(|result| result.timed_out);
        trace.append(
            "checks",
            if all_pass { "passed" } else { "failed" },
            json!({ "implementation_turn": receipt.implementations.len(), "results": results }),
        )?;
        receipt.check_attempts.push(CheckAttempt {
            implementation_turn: receipt.implementations.len(),
            results: results.clone(),
        });
        verify_integrity(context)?;
        if check_mutated_source {
            return Err(SheprdError::Message(
                "check command mutated non-ignored source state".into(),
            ));
        }
        if check_timed_out {
            return Err(SheprdError::Message(
                "check command exceeded the factory timeout".into(),
            ));
        }
        if all_pass {
            break;
        }
        if correction_turn == MAX_CORRECTION_TURNS {
            return Err(SheprdError::Message(
                "checks still fail after two Codex correction turns".into(),
            ));
        }
        let correction_marker = EnvelopeMarker::fresh()?;
        let correction_prompt = implementation_prompt(
            context.request,
            &plan,
            correction_turn + 1,
            Some(&results),
            &correction_marker,
        )?;
        let correction_result = run_agent_phase(
            codex,
            &correction_prompt,
            "implementation_correction",
            trace,
        );
        require_ignored_state_unchanged(&worker_path, &post_check_ignored, "Codex correction")?;
        let correction_text = correction_result?;
        let correction: ImplementationEnvelope =
            parse_envelope(&correction_text, "implementation", &correction_marker)?;
        verify_integrity(context)?;
        validate_implementation(&correction, &context.allowed)?;
        trace.append(
            "implementation_correction",
            "parsed",
            serde_json::to_value(&correction)?,
        )?;
        receipt.implementations.push(correction);
    }

    let review_source = source_snapshot(&worker_path, &worker_head)?;
    let actual_paths = review_source.paths.clone();
    if actual_paths.is_empty() {
        return Err(SheprdError::Message(
            "Codex implementation produced no changed paths".into(),
        ));
    }
    let final_implementation = receipt
        .implementations
        .last()
        .ok_or_else(|| SheprdError::Message("factory implementation is missing".into()))?;
    verify_claimed_changed_paths(final_implementation, &actual_paths)?;
    let patch = review_patch(&worker_path, &worker_head, &actual_paths)?;
    require_source_snapshot_unchanged(
        &worker_path,
        &worker_head,
        &review_source,
        "review patch construction",
    )?;
    let final_checks = receipt
        .check_attempts
        .last()
        .map(|attempt| &attempt.results)
        .ok_or_else(|| SheprdError::Message("factory checks did not run".into()))?;

    let claude_marker = EnvelopeMarker::fresh()?;
    let claude_prompt = review_prompt(
        "claude",
        "intent review: verify the implementation matches the typed plan and task",
        context.request,
        &plan,
        final_checks,
        &actual_paths,
        &patch,
        &claude_marker,
    )?;
    let claude_result = run_agent_phase(claude, &claude_prompt, "claude_review", trace);
    if git_snapshot(Path::new(&claude.cwd))? != claude_before {
        return Err(SheprdError::Message(
            "Claude review modified its checkout".into(),
        ));
    }
    let claude_text = claude_result?;
    let claude_review: ReviewEnvelope = parse_envelope(&claude_text, "review", &claude_marker)?;
    validate_reviewer(&claude_review, "claude")?;
    trace.append(
        "claude_review",
        if claude_review.approved {
            "approved"
        } else {
            "rejected"
        },
        serde_json::to_value(&claude_review)?,
    )?;
    receipt.claude_review = Some(claude_review);
    require_source_snapshot_unchanged(&worker_path, &worker_head, &review_source, "Claude review")?;
    verify_integrity(context)?;

    let opencode_marker = EnvelopeMarker::fresh()?;
    let opencode_prompt = review_prompt(
        "opencode",
        "adversarial review: search for correctness, safety, scope, and test gaps",
        context.request,
        &plan,
        final_checks,
        &actual_paths,
        &patch,
        &opencode_marker,
    )?;
    let opencode_result = run_agent_phase(opencode, &opencode_prompt, "opencode_review", trace);
    if git_snapshot(Path::new(&opencode.cwd))? != opencode_before {
        return Err(SheprdError::Message(
            "OpenCode review modified its checkout".into(),
        ));
    }
    let opencode_text = opencode_result?;
    let opencode_review: ReviewEnvelope =
        parse_envelope(&opencode_text, "review", &opencode_marker)?;
    validate_reviewer(&opencode_review, "opencode")?;
    trace.append(
        "opencode_review",
        if opencode_review.approved {
            "approved"
        } else {
            "rejected"
        },
        serde_json::to_value(&opencode_review)?,
    )?;
    receipt.opencode_review = Some(opencode_review);
    require_source_snapshot_unchanged(
        &worker_path,
        &worker_head,
        &review_source,
        "OpenCode review",
    )?;
    let final_paths = changed_paths(&worker_path, &worker_head)?;
    verify_claimed_changed_paths(final_implementation, &final_paths)?;
    verify_integrity(context)?;
    Ok(())
}

fn agent<'a>(agents: &'a [FlokAgent], kind: &str) -> Result<&'a FlokAgent> {
    agents
        .iter()
        .find(|agent| agent.kind == kind)
        .ok_or_else(|| SheprdError::Message(format!("healthy Flok is missing {kind}")))
}

fn run_agent_phase(
    agent: &FlokAgent,
    prompt: &str,
    phase: &str,
    trace: &mut TraceWriter,
) -> Result<String> {
    require_agent_prompt_size(prompt)?;
    trace.append(
        phase,
        "prompted",
        json!({ "agent": agent.name, "kind": agent.kind }),
    )?;
    herdr::prompt_agent(&agent.name, prompt)?;
    herdr::wait_for_agent(&agent.name)?;
    let text = herdr::read_agent(&agent.name)?;
    trace.append(
        phase,
        "responded",
        json!({ "agent": agent.name, "bytes": text.len() }),
    )?;
    Ok(text)
}

fn require_agent_prompt_size(prompt: &str) -> Result<()> {
    if prompt.len() > MAX_AGENT_PROMPT_BYTES {
        return Err(SheprdError::Message(format!(
            "agent prompt exceeds the {MAX_AGENT_PROMPT_BYTES} byte factory limit"
        )));
    }
    Ok(())
}

impl EnvelopeMarker {
    fn fresh() -> Result<Self> {
        let mut bytes = [0_u8; 32];
        File::open("/dev/urandom")?.read_exact(&mut bytes)?;
        let nonce = bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        Ok(Self::from_nonce(nonce))
    }

    fn from_nonce(nonce: impl Into<String>) -> Self {
        let nonce = nonce.into();
        Self {
            start: format!("{MARKER_START_PREFIX}{nonce}>>>"),
            end: format!("{MARKER_END_PREFIX}{nonce}>>>"),
            nonce,
        }
    }

    fn instructions(&self) -> String {
        // Keep parseable markers out of the prompt so a prompt echo cannot
        // satisfy the response contract. The agent constructs them itself.
        format!(
            "Envelope nonce: {}\nConstruct the start marker by concatenating these JSON strings: [\"<<<SHEPRD_\",\"FACTORY_JSON_START:\",\"{}\",\">>>\"]. Construct the end marker by concatenating: [\"<<<SHEPRD_\",\"FACTORY_JSON_END:\",\"{}\",\">>>\"]. Emit exactly one start marker, one JSON object, and one end marker, with no other marker-like text.",
            self.nonce, self.nonce, self.nonce
        )
    }
}

fn plan_prompt(request: &FactoryRequest, marker: &EnvelopeMarker) -> String {
    format!(
        "Factory plan phase. Do not edit files or delegate.\nTask: {}\nAllowed paths: {}\nChecks: {}\nRequired JSON fields: {{\"schema_version\":1,\"kind\":\"plan\",\"nonce\":\"{}\",\"summary\":\"...\",\"steps\":[{{\"id\":\"P1\",\"objective\":\"...\",\"allow_paths\":[\"...\"]}}]}}\n{}",
        escape_prompt_content(&request.task),
        escape_prompt_content(&request.allow_paths.join(", ")),
        escape_prompt_content(&request.checks.join("; ")),
        marker.nonce,
        marker.instructions(),
    )
}

fn implementation_prompt(
    request: &FactoryRequest,
    plan: &PlanEnvelope,
    correction_turn: usize,
    failures: Option<&[CheckResult]>,
    marker: &EnvelopeMarker,
) -> Result<String> {
    let phase = if correction_turn == 0 {
        "Initial implementation turn".to_string()
    } else {
        format!("Correction turn {correction_turn} of {MAX_CORRECTION_TURNS}")
    };
    let failure_json = failures
        .map(serde_json::to_string_pretty)
        .transpose()?
        .unwrap_or_else(|| "[]".into());
    Ok(format!(
        "{phase}. Implement the task only in your current worker checkout. Do not delegate, commit, merge, push, or modify paths outside the allow list. Rust, not an agent, runs checks.\nTask: {}\nAllowed paths: {}\nTyped plan: {}\nPrevious check failures: {}\nRequired JSON fields: {{\"schema_version\":1,\"kind\":\"implementation\",\"nonce\":\"{}\",\"summary\":\"...\",\"claimed_changed_paths\":[\"...\"]}}\n{}",
        escape_prompt_content(&request.task),
        escape_prompt_content(&request.allow_paths.join(", ")),
        escape_prompt_content(&serde_json::to_string(plan)?),
        escape_prompt_content(&failure_json),
        marker.nonce,
        marker.instructions(),
    ))
}

fn review_prompt(
    reviewer: &str,
    purpose: &str,
    request: &FactoryRequest,
    plan: &PlanEnvelope,
    checks: &[CheckResult],
    changed_paths: &[String],
    patch: &str,
    marker: &EnvelopeMarker,
) -> Result<String> {
    Ok(format!(
        "Factory {purpose}. Review only; do not edit, delegate, commit, merge, or push. Fail closed on ambiguity.\nTask: {}\nAllowed paths: {}\nPlan: {}\nActual changed paths: {}\nRust check results: {}\nPatch:\n{}\nRequired JSON fields: {{\"schema_version\":1,\"kind\":\"review\",\"nonce\":\"{}\",\"reviewer\":\"{reviewer}\",\"approved\":false,\"summary\":\"...\",\"findings\":[\"...\"]}}\n{}",
        escape_prompt_content(&request.task),
        escape_prompt_content(&request.allow_paths.join(", ")),
        escape_prompt_content(&serde_json::to_string(plan)?),
        escape_prompt_content(&changed_paths.join(", ")),
        escape_prompt_content(&serde_json::to_string(checks)?),
        escape_prompt_content(patch),
        marker.nonce,
        marker.instructions(),
    ))
}

fn escape_prompt_content(value: &str) -> String {
    value
        .replace(MARKER_LIKE_PREFIX, "[redacted Sheprd factory marker]")
        .replace(
            LEGACY_MARKER_LIKE_PREFIX,
            "[redacted Sheprd factory marker]",
        )
}

fn parse_envelope<T: DeserializeOwned>(
    text: &str,
    expected_kind: &str,
    marker: &EnvelopeMarker,
) -> Result<T> {
    if text.matches(MARKER_START_PREFIX).count() != 1
        || text.matches(MARKER_END_PREFIX).count() != 1
    {
        return Err(SheprdError::Message(
            "agent response must contain exactly one factory envelope pair".into(),
        ));
    }
    let start = text.find(&marker.start).ok_or_else(|| {
        SheprdError::Message("agent response has a stale or mismatched envelope nonce".into())
    })?;
    let body_start = start + marker.start.len();
    let relative_end = text[body_start..].find(&marker.end).ok_or_else(|| {
        SheprdError::Message("agent response has a stale or mismatched envelope nonce".into())
    })?;
    let end = body_start + relative_end;
    let before = &text[..start];
    let body = &text[body_start..end];
    let after = &text[end + marker.end.len()..];
    if [before, body, after]
        .iter()
        .any(|part| part.contains(MARKER_LIKE_PREFIX) || part.contains(LEGACY_MARKER_LIKE_PREFIX))
    {
        return Err(SheprdError::Message(
            "agent response contains nested or extra factory markers".into(),
        ));
    }
    let value: Value = serde_json::from_str(body.trim()).map_err(|error| {
        SheprdError::Message(format!("agent envelope is not valid JSON: {error}"))
    })?;
    if value.get("schema_version").and_then(Value::as_u64)
        != Some(u64::from(FACTORY_SCHEMA_VERSION))
    {
        return Err(SheprdError::Message(
            "agent envelope has an unsupported schema_version".into(),
        ));
    }
    if value.get("kind").and_then(Value::as_str) != Some(expected_kind) {
        return Err(SheprdError::Message(format!(
            "agent envelope kind must be {expected_kind}"
        )));
    }
    if value.get("nonce").and_then(Value::as_str) != Some(marker.nonce.as_str()) {
        return Err(SheprdError::Message(
            "agent envelope nonce does not match its markers".into(),
        ));
    }
    serde_json::from_value(value).map_err(Into::into)
}

fn validate_plan(plan: &PlanEnvelope, allowed: &[PathBuf]) -> Result<()> {
    if plan.summary.trim().is_empty() || plan.steps.is_empty() {
        return Err(SheprdError::Message(
            "typed plan must include a summary and at least one step".into(),
        ));
    }
    for step in &plan.steps {
        if step.id.trim().is_empty() || step.objective.trim().is_empty() {
            return Err(SheprdError::Message(
                "typed plan steps require id and objective".into(),
            ));
        }
        let step_paths = normalize_allow_paths(&step.allow_paths)?;
        if let Some(path) = step_paths
            .iter()
            .find(|path| !path_is_allowed(path, allowed))
        {
            return Err(SheprdError::Message(format!(
                "typed plan exceeds caller allow paths: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

fn validate_reviewer(review: &ReviewEnvelope, expected: &str) -> Result<()> {
    if review.reviewer != expected {
        return Err(SheprdError::Message(format!(
            "review envelope must identify reviewer {expected}"
        )));
    }
    if review.summary.trim().is_empty() {
        return Err(SheprdError::Message(
            "review envelope summary must not be empty".into(),
        ));
    }
    Ok(())
}

fn validate_implementation(
    implementation: &ImplementationEnvelope,
    allowed: &[PathBuf],
) -> Result<()> {
    if implementation.summary.trim().is_empty() || implementation.claimed_changed_paths.is_empty() {
        return Err(SheprdError::Message(
            "implementation envelope requires a summary and claimed changed paths".into(),
        ));
    }
    for path in &implementation.claimed_changed_paths {
        let normalized = normalize_relative_path(Path::new(path))?;
        if !path_is_allowed(&normalized, allowed) {
            return Err(SheprdError::Message(format!(
                "implementation envelope claims an out-of-scope path: {path}"
            )));
        }
    }
    Ok(())
}

fn verify_claimed_changed_paths(
    implementation: &ImplementationEnvelope,
    actual_paths: &[String],
) -> Result<()> {
    let claimed = implementation
        .claimed_changed_paths
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let actual = actual_paths.iter().cloned().collect::<BTreeSet<_>>();
    if claimed != actual {
        return Err(SheprdError::Message(format!(
            "Codex claimed changed paths do not match Git: claimed={claimed:?} actual={actual:?}"
        )));
    }
    Ok(())
}

fn require_source_snapshot_unchanged(
    cwd: &Path,
    initial_head: &str,
    expected: &SourceSnapshot,
    phase: &str,
) -> Result<()> {
    if &source_snapshot(cwd, initial_head)? != expected {
        return Err(SheprdError::Message(format!(
            "Codex worker source changed during {phase}"
        )));
    }
    Ok(())
}

fn verify_integrity(context: &RunContext<'_>) -> Result<()> {
    if git_snapshot(&context.project.path)? != context.base_before {
        return Err(SheprdError::Message(
            "base checkout changed during the factory run".into(),
        ));
    }
    if let (Some(worker_path), Some(worker_head)) = (
        context.worker_path.as_deref(),
        context.worker_head.as_deref(),
    ) {
        if git_head(worker_path)? != worker_head {
            return Err(SheprdError::Message(
                "Codex worker HEAD changed; factory runs must not commit".into(),
            ));
        }
        let paths = changed_paths(worker_path, worker_head)?;
        if let Some(path) = first_disallowed_path(&paths, &context.allowed)? {
            return Err(SheprdError::Message(format!(
                "changed path is outside the declared allow paths: {path}"
            )));
        }
    }
    Ok(())
}

fn normalize_allow_paths(paths: &[String]) -> Result<Vec<PathBuf>> {
    if paths.is_empty() {
        return Err(SheprdError::Message(
            "factory requires at least one --allow-path".into(),
        ));
    }
    paths
        .iter()
        .map(|value| normalize_relative_path(Path::new(value)))
        .collect()
}

fn normalize_relative_path(path: &Path) -> Result<PathBuf> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(SheprdError::Message(format!(
            "allow paths must be non-empty and repository-relative: {}",
            path.display()
        )));
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => normalized.push(value),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(SheprdError::Message(format!(
                    "allow path escapes the repository: {}",
                    path.display()
                )))
            }
        }
    }
    if normalized.as_os_str().is_empty() {
        normalized.push(".");
    }
    Ok(normalized)
}

fn path_is_allowed(path: &Path, allowed: &[PathBuf]) -> bool {
    allowed
        .iter()
        .any(|allowed| allowed == Path::new(".") || path == allowed || path.starts_with(allowed))
}

fn first_disallowed_path(paths: &[String], allowed: &[PathBuf]) -> Result<Option<String>> {
    for path in paths {
        let normalized = normalize_relative_path(Path::new(path))?;
        if !path_is_allowed(&normalized, allowed) {
            return Ok(Some(path.clone()));
        }
    }
    Ok(None)
}

fn run_checks(
    cwd: &Path,
    initial_head: &str,
    commands: &[String],
    timeout: Duration,
) -> Result<Vec<CheckResult>> {
    let mut results = Vec::new();
    for command in commands {
        let result = run_check(cwd, initial_head, command, timeout)?;
        let terminal = result.timed_out || result.mutated_source;
        results.push(result);
        if terminal {
            break;
        }
    }
    Ok(results)
}

fn run_check(
    cwd: &Path,
    initial_head: &str,
    command: &str,
    timeout: Duration,
) -> Result<CheckResult> {
    let before = source_snapshot(cwd, initial_head)?;
    let started = Instant::now();
    let mut process = Command::new("/bin/sh");
    process
        .args(["-c", command])
        .current_dir(cwd)
        .env_clear()
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    for key in CHECK_ENV_ALLOWLIST {
        if let Some(value) = std::env::var_os(key) {
            process.env(key, value);
        }
    }
    if std::env::var_os("PATH").is_none() {
        process.env("PATH", "/usr/bin:/bin");
    }
    process.env("SHEPRD_FACTORY_CHECK", "1");

    let mut child = match process.spawn() {
        Ok(child) => child,
        Err(error) => {
            let (mutated_source, mutation_error) =
                compare_source_snapshot(cwd, initial_head, &before);
            let mut stderr = error.to_string();
            if let Some(mutation_error) = mutation_error {
                stderr.push_str(&format!(
                    "\n[source snapshot failed closed: {mutation_error}]"
                ));
            }
            return Ok(CheckResult {
                command: command.into(),
                success: false,
                exit_code: None,
                timed_out: false,
                mutated_source,
                duration_ms: started.elapsed().as_millis(),
                stdout: String::new(),
                stderr,
            });
        }
    };
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| SheprdError::Message("check stdout pipe is unavailable".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| SheprdError::Message("check stderr pipe is unavailable".into()))?;
    let stdout_reader = std::thread::spawn(move || read_capped(stdout));
    let stderr_reader = std::thread::spawn(move || read_capped(stderr));
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed() >= timeout {
            timed_out = true;
            kill_process_group(child.id());
            let _ = child.kill();
            break child.wait()?;
        }
        std::thread::sleep(CHECK_POLL_INTERVAL);
    };
    let stdout = stdout_reader
        .join()
        .map_err(|_| SheprdError::Message("check stdout reader panicked".into()))??;
    let stderr = stderr_reader
        .join()
        .map_err(|_| SheprdError::Message("check stderr reader panicked".into()))??;
    let (mutated_source, mutation_error) = compare_source_snapshot(cwd, initial_head, &before);
    let mut stderr = capture(&stderr);
    if timed_out {
        stderr.push_str(&format!(
            "\n[check timed out after {} ms]",
            timeout.as_millis()
        ));
    }
    if mutated_source {
        stderr.push_str("\n[check mutated non-ignored source state]");
    }
    if let Some(mutation_error) = mutation_error {
        stderr.push_str(&format!(
            "\n[source snapshot failed closed: {mutation_error}]"
        ));
    }
    Ok(CheckResult {
        command: command.into(),
        success: status.success() && !timed_out && !mutated_source,
        exit_code: status.code(),
        timed_out,
        mutated_source,
        duration_ms: started.elapsed().as_millis(),
        stdout: capture(&stdout),
        stderr,
    })
}

fn compare_source_snapshot(
    cwd: &Path,
    initial_head: &str,
    before: &SourceSnapshot,
) -> (bool, Option<String>) {
    match source_snapshot(cwd, initial_head) {
        Ok(after) => (before != &after, None),
        Err(error) => (true, Some(error.to_string())),
    }
}

fn read_capped(reader: impl Read) -> std::io::Result<Vec<u8>> {
    read_bounded(reader, MAX_CAPTURE_BYTES)
}

fn read_bounded(mut reader: impl Read, max_bytes: usize) -> std::io::Result<Vec<u8>> {
    let mut captured = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        if captured.len() <= max_bytes {
            let remaining = max_bytes + 1 - captured.len();
            captured.extend_from_slice(&buffer[..count.min(remaining)]);
        }
    }
    Ok(captured)
}

fn kill_process_group(pid: u32) {
    const SIGKILL: i32 = 9;
    unsafe extern "C" {
        fn kill(pid: i32, signal: i32) -> i32;
    }
    if let Ok(pid) = i32::try_from(pid) {
        // The child created a fresh process group whose id is its pid.
        unsafe {
            kill(-pid, SIGKILL);
        }
    }
}

fn capture(bytes: &[u8]) -> String {
    let truncated = bytes.len() > MAX_CAPTURE_BYTES;
    let end = bytes.len().min(MAX_CAPTURE_BYTES);
    let mut output = String::from_utf8_lossy(&bytes[..end]).into_owned();
    if truncated {
        output.push_str("\n[output truncated by Sheprd]");
    }
    output
}

fn git_snapshot(cwd: &Path) -> Result<GitSnapshot> {
    Ok(GitSnapshot {
        head: git_head(cwd)?,
        status: git_text(cwd, &["status", "--porcelain", "--untracked-files=all"])?,
        ignored: ignored_state_snapshot(cwd)?,
    })
}

fn git_head(cwd: &Path) -> Result<String> {
    Ok(git_text(cwd, &["rev-parse", "HEAD"])?.trim().to_string())
}

fn git_text(cwd: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git").args(args).current_dir(cwd).output()?;
    if !output.status.success() {
        return Err(SheprdError::Message(
            String::from_utf8_lossy(&output.stderr).trim().into(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn git_nul_paths(cwd: &Path, args: &[&str]) -> Result<Vec<String>> {
    let output = Command::new("git").args(args).current_dir(cwd).output()?;
    if !output.status.success() {
        return Err(SheprdError::Message(
            String::from_utf8_lossy(&output.stderr).trim().into(),
        ));
    }
    output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| {
            String::from_utf8(path.to_vec()).map_err(|_| {
                SheprdError::Message("factory does not support non-UTF-8 changed paths".into())
            })
        })
        .collect()
}

fn git_nul_paths_bounded(cwd: &Path, args: &[&str], description: &str) -> Result<Vec<String>> {
    let mut child = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| SheprdError::Message(format!("{description} stdout is unavailable")))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| SheprdError::Message(format!("{description} stderr is unavailable")))?;
    let stdout_reader = std::thread::spawn(move || read_bounded(stdout, MAX_IGNORED_ENUM_BYTES));
    let stderr_reader = std::thread::spawn(move || read_capped(stderr));
    let status = child.wait()?;
    let stdout = stdout_reader
        .join()
        .map_err(|_| SheprdError::Message(format!("{description} stdout reader panicked")))??;
    let stderr = stderr_reader
        .join()
        .map_err(|_| SheprdError::Message(format!("{description} stderr reader panicked")))??;
    if stdout.len() > MAX_IGNORED_ENUM_BYTES {
        return Err(SheprdError::Message(format!(
            "{description} exceeds the {MAX_IGNORED_ENUM_BYTES} byte enumeration limit"
        )));
    }
    if !status.success() {
        return Err(SheprdError::Message(format!(
            "{description} failed: {}",
            capture(&stderr).trim()
        )));
    }
    stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| {
            String::from_utf8(path.to_vec()).map_err(|_| {
                SheprdError::Message(format!("{description} returned a non-UTF-8 path"))
            })
        })
        .collect()
}

fn changed_paths(cwd: &Path, initial_head: &str) -> Result<Vec<String>> {
    let mut paths = BTreeSet::new();
    for path in git_nul_paths(
        cwd,
        &[
            "diff",
            "--no-renames",
            "--name-only",
            "-z",
            initial_head,
            "--",
        ],
    )? {
        paths.insert(path);
    }
    for path in git_nul_paths(
        cwd,
        &["ls-files", "--others", "--exclude-standard", "-z", "--"],
    )? {
        paths.insert(path);
    }
    Ok(paths.into_iter().collect())
}

fn file_metadata_snapshot(
    metadata: &std::fs::Metadata,
    description: &str,
) -> Result<FileMetadataSnapshot> {
    let kind = if metadata.file_type().is_file() {
        FileKind::Regular
    } else if metadata.file_type().is_dir() {
        FileKind::Directory
    } else if metadata.file_type().is_symlink() {
        FileKind::Symlink
    } else {
        return Err(SheprdError::Message(format!(
            "{description} has an unsupported filesystem type"
        )));
    };
    Ok(FileMetadataSnapshot {
        kind,
        len: metadata.len(),
        mode: metadata.mode(),
        device: metadata.dev(),
        inode: metadata.ino(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
    })
}

fn require_same_metadata(
    before: &FileMetadataSnapshot,
    after: &FileMetadataSnapshot,
    description: &str,
) -> Result<()> {
    if before != after {
        return Err(SheprdError::Message(format!(
            "{description} changed while factory state was being read"
        )));
    }
    Ok(())
}

fn ignored_paths(cwd: &Path) -> Result<Vec<String>> {
    let mut paths = BTreeSet::new();
    for args in [
        &[
            "ls-files",
            "--others",
            "--ignored",
            "--exclude-standard",
            "-z",
            "--",
        ][..],
        &[
            "ls-files",
            "--others",
            "--ignored",
            "--exclude-standard",
            "--directory",
            "-z",
            "--",
        ][..],
    ] {
        for path in git_nul_paths_bounded(cwd, args, "ignored-state enumeration")? {
            let normalized = normalize_relative_path(Path::new(&path))?;
            let normalized = normalized.to_str().ok_or_else(|| {
                SheprdError::Message("ignored-state path is not valid UTF-8".into())
            })?;
            paths.insert(normalized.to_string());
            if paths.len() > MAX_IGNORED_ENTRIES {
                return Err(SheprdError::Message(format!(
                    "ignored-state snapshot exceeds the {MAX_IGNORED_ENTRIES} entry limit"
                )));
            }
        }
    }
    let path_bytes = paths.iter().try_fold(0_usize, |total, path| {
        total
            .checked_add(path.len())
            .ok_or_else(|| SheprdError::Message("ignored-state path byte count overflowed".into()))
    })?;
    if path_bytes > MAX_IGNORED_ENUM_BYTES {
        return Err(SheprdError::Message(format!(
            "ignored-state paths exceed the {MAX_IGNORED_ENUM_BYTES} byte limit"
        )));
    }
    Ok(paths.into_iter().collect())
}

fn ignored_state_once(cwd: &Path) -> Result<IgnoredStateSnapshot> {
    let paths = ignored_paths(cwd)?;
    let mut entries = Vec::with_capacity(paths.len());
    let mut total_file_bytes = 0_u64;
    for path in paths {
        let metadata = std::fs::symlink_metadata(cwd.join(&path)).map_err(|error| {
            SheprdError::Message(format!("cannot inspect ignored-state path {path}: {error}"))
        })?;
        let metadata = file_metadata_snapshot(&metadata, "ignored-state path")?;
        if metadata.kind != FileKind::Directory {
            total_file_bytes = total_file_bytes
                .checked_add(metadata.len)
                .ok_or_else(|| SheprdError::Message("ignored-state file size overflowed".into()))?;
            if total_file_bytes > MAX_IGNORED_TOTAL_FILE_BYTES {
                return Err(SheprdError::Message(format!(
                    "ignored-state files exceed the {MAX_IGNORED_TOTAL_FILE_BYTES} byte limit"
                )));
            }
        }
        entries.push((path, metadata));
    }
    Ok(IgnoredStateSnapshot {
        entries,
        total_file_bytes,
    })
}

fn ignored_state_snapshot(cwd: &Path) -> Result<IgnoredStateSnapshot> {
    let before = ignored_state_once(cwd)?;
    let after = ignored_state_once(cwd)?;
    if before != after {
        return Err(SheprdError::Message(
            "ignored state changed while it was being snapshotted".into(),
        ));
    }
    Ok(before)
}

fn require_ignored_state_unchanged(
    cwd: &Path,
    expected: &IgnoredStateSnapshot,
    actor: &str,
) -> Result<()> {
    if &ignored_state_snapshot(cwd)? != expected {
        return Err(SheprdError::Message(format!(
            "{actor} mutated ignored worktree state"
        )));
    }
    Ok(())
}

fn git_admin_snapshot(cwd: &Path) -> Result<GitAdminSnapshot> {
    let path = cwd.join(".git");
    let path_metadata = std::fs::symlink_metadata(&path).map_err(|error| {
        SheprdError::Message(format!(
            "cannot inspect worktree .git administrative entry: {error}"
        ))
    })?;
    let expected = file_metadata_snapshot(&path_metadata, "worktree .git administrative entry")?;
    if expected.kind == FileKind::Symlink {
        return Err(SheprdError::Message(
            "worktree .git administrative entry must not be a symlink".into(),
        ));
    }
    if expected.kind == FileKind::Directory {
        let after = file_metadata_snapshot(
            &std::fs::symlink_metadata(&path)?,
            "worktree .git administrative entry",
        )?;
        require_same_metadata(&expected, &after, "worktree .git administrative entry")?;
        return Ok(GitAdminSnapshot {
            metadata: expected,
            content_digest: None,
        });
    }
    if expected.len > u64::try_from(MAX_GIT_ADMIN_ENTRY_BYTES).unwrap_or(u64::MAX) {
        return Err(SheprdError::Message(format!(
            "worktree .git administrative entry exceeds {MAX_GIT_ADMIN_ENTRY_BYTES} bytes"
        )));
    }
    let mut file = File::open(&path).map_err(|error| {
        SheprdError::Message(format!(
            "cannot read worktree .git administrative entry: {error}"
        ))
    })?;
    let opened = file_metadata_snapshot(
        &file.metadata()?,
        "worktree .git administrative entry handle",
    )?;
    require_same_metadata(&expected, &opened, "worktree .git administrative entry")?;
    let mut contents = Vec::with_capacity(usize::try_from(expected.len).unwrap_or(0));
    (&mut file)
        .take(
            u64::try_from(MAX_GIT_ADMIN_ENTRY_BYTES)
                .unwrap_or(u64::MAX)
                .saturating_add(1),
        )
        .read_to_end(&mut contents)?;
    if contents.len() > MAX_GIT_ADMIN_ENTRY_BYTES {
        return Err(SheprdError::Message(format!(
            "worktree .git administrative entry exceeds {MAX_GIT_ADMIN_ENTRY_BYTES} bytes"
        )));
    }
    let handle_after = file_metadata_snapshot(
        &file.metadata()?,
        "worktree .git administrative entry handle",
    )?;
    require_same_metadata(&opened, &handle_after, "worktree .git administrative entry")?;
    let path_after = file_metadata_snapshot(
        &std::fs::symlink_metadata(&path)?,
        "worktree .git administrative entry",
    )?;
    require_same_metadata(
        &handle_after,
        &path_after,
        "worktree .git administrative entry",
    )?;
    Ok(GitAdminSnapshot {
        metadata: opened,
        content_digest: Some(format!("{:x}", Sha256::digest(&contents))),
    })
}

fn source_snapshot(cwd: &Path, initial_head: &str) -> Result<SourceSnapshot> {
    let git_admin = git_admin_snapshot(cwd)?;
    let paths = changed_paths(cwd, initial_head)?;
    let mut digest = Sha256::new();
    digest.update(initial_head.as_bytes());
    for path in &paths {
        digest.update(path.as_bytes());
        digest.update([0]);
    }
    let diff = Command::new("git")
        .args(["diff", "--no-ext-diff", "--binary", initial_head, "--"])
        .current_dir(cwd)
        .output()?;
    if !diff.status.success() {
        return Err(SheprdError::Message(
            String::from_utf8_lossy(&diff.stderr).trim().into(),
        ));
    }
    digest.update(&diff.stdout);
    let untracked = git_nul_paths(
        cwd,
        &["ls-files", "--others", "--exclude-standard", "-z", "--"],
    )?;
    for path in untracked {
        digest.update(path.as_bytes());
        digest.update([0]);
        let full_path = cwd.join(&path);
        let metadata = std::fs::symlink_metadata(&full_path)?;
        digest.update(metadata.permissions().mode().to_le_bytes());
        if metadata.file_type().is_symlink() {
            digest.update(b"symlink\0");
            digest.update(
                std::fs::read_link(full_path)?
                    .as_os_str()
                    .as_encoded_bytes(),
            );
        } else if metadata.is_file() {
            digest.update(b"file\0");
            let mut file = File::open(full_path)?;
            let mut buffer = [0_u8; 8192];
            loop {
                let count = file.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                digest.update(&buffer[..count]);
            }
        } else {
            return Err(SheprdError::Message(format!(
                "untracked source path must be a file or symlink: {path}"
            )));
        }
    }
    let git_admin_after = git_admin_snapshot(cwd)?;
    if git_admin != git_admin_after {
        return Err(SheprdError::Message(
            "worktree .git administrative entry changed during Git state inspection".into(),
        ));
    }
    Ok(SourceSnapshot {
        git_admin,
        paths,
        digest: format!("{:x}", digest.finalize()),
    })
}

fn read_untracked_review_file(
    full_path: &Path,
    path: &str,
    expected: &FileMetadataSnapshot,
) -> Result<Vec<u8>> {
    let mut file = File::open(full_path)?;
    let opened = file_metadata_snapshot(&file.metadata()?, "untracked review file handle")?;
    if opened.kind != FileKind::Regular {
        return Err(SheprdError::Message(format!(
            "untracked review path must be a regular file: {path}"
        )));
    }
    require_same_metadata(expected, &opened, "untracked review file")?;
    let capacity = usize::try_from(opened.len)
        .map_err(|_| SheprdError::Message(format!("untracked review file is too large: {path}")))?;
    let mut contents = Vec::with_capacity(capacity);
    (&mut file)
        .take(opened.len.saturating_add(1))
        .read_to_end(&mut contents)?;
    let handle_after = file_metadata_snapshot(&file.metadata()?, "untracked review file handle")?;
    require_same_metadata(&opened, &handle_after, "untracked review file")?;
    let path_after = file_metadata_snapshot(
        &std::fs::symlink_metadata(full_path)?,
        "untracked review path",
    )?;
    require_same_metadata(&handle_after, &path_after, "untracked review path")?;
    if contents.len() != capacity {
        return Err(SheprdError::Message(format!(
            "untracked review file changed while being read: {path}"
        )));
    }
    Ok(contents)
}

fn review_patch(cwd: &Path, initial_head: &str, changed_paths: &[String]) -> Result<String> {
    let output = Command::new("git")
        .args(["diff", "--no-ext-diff", "--binary", initial_head, "--"])
        .current_dir(cwd)
        .output()?;
    if !output.status.success() {
        return Err(SheprdError::Message(
            String::from_utf8_lossy(&output.stderr).trim().into(),
        ));
    }
    let mut patch = output.stdout;
    if patch.len() > MAX_REVIEW_PATCH_BYTES {
        return Err(SheprdError::Message(format!(
            "review patch exceeds the {MAX_REVIEW_PATCH_BYTES} byte factory limit"
        )));
    }
    let tracked = git_nul_paths(
        cwd,
        &[
            "diff",
            "--no-renames",
            "--name-only",
            "-z",
            initial_head,
            "--",
        ],
    )?
    .into_iter()
    .collect::<BTreeSet<_>>();
    for path in changed_paths.iter().filter(|path| !tracked.contains(*path)) {
        let full_path = cwd.join(path);
        let metadata = std::fs::symlink_metadata(&full_path)?;
        if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
            return Err(SheprdError::Message(format!(
                "untracked review path must be a regular file: {path}"
            )));
        }
        let expected = file_metadata_snapshot(&metadata, "untracked review path")?;
        let header = format!("\n--- /dev/null\n+++ b/{path}\n");
        let file_size = usize::try_from(metadata.len()).map_err(|_| {
            SheprdError::Message(format!("untracked review file is too large: {path}"))
        })?;
        let projected = patch
            .len()
            .checked_add(header.len())
            .and_then(|size| size.checked_add(file_size))
            .and_then(|size| size.checked_add(1))
            .ok_or_else(|| SheprdError::Message("review patch size overflow".into()))?;
        if projected > MAX_REVIEW_PATCH_BYTES {
            return Err(SheprdError::Message(format!(
                "review patch exceeds the {MAX_REVIEW_PATCH_BYTES} byte factory limit before reading untracked file {path}"
            )));
        }
        let contents = read_untracked_review_file(&full_path, path, &expected)?;
        patch.extend_from_slice(header.as_bytes());
        patch.extend_from_slice(&contents);
        patch.push(b'\n');
    }
    if patch.len() > MAX_REVIEW_PATCH_BYTES {
        return Err(SheprdError::Message(format!(
            "review patch exceeds the {} byte factory limit",
            MAX_REVIEW_PATCH_BYTES
        )));
    }
    Ok(String::from_utf8_lossy(&patch).into_owned())
}

fn factory_state_root(project: &Project) -> Result<PathBuf> {
    let state_root = if let Some(path) = std::env::var_os("SHEPRD_STATE_DIR") {
        PathBuf::from(path)
    } else {
        let home = std::env::var_os("HOME").ok_or(SheprdError::MissingHome)?;
        PathBuf::from(home).join(".local/state/sheprd")
    };
    let factory_root = state_root.join("factory");
    create_private_dir(&factory_root)?;
    let project_root = factory_root.join(short_hash(&project.path));
    create_private_dir(&project_root)?;
    Ok(project_root)
}

fn create_private_dir(path: &Path) -> Result<()> {
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    Ok(())
}

fn factory_run_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{nanos}-{}", std::process::id())
}

fn short_hash(path: &Path) -> String {
    let digest = Sha256::digest(path.to_string_lossy().as_bytes());
    digest[..4]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[derive(Serialize)]
struct TraceEvent<'a> {
    schema_version: u32,
    sequence: u64,
    phase: &'a str,
    status: &'a str,
    detail: Value,
}

struct TraceWriter {
    file: File,
    sequence: u64,
}

impl TraceWriter {
    fn create(path: &Path) -> Result<Self> {
        let file = OpenOptions::new()
            .append(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?;
        Ok(Self { file, sequence: 0 })
    }

    fn append(&mut self, phase: &str, status: &str, detail: Value) -> Result<()> {
        self.sequence += 1;
        serde_json::to_writer(
            &mut self.file,
            &TraceEvent {
                schema_version: FACTORY_SCHEMA_VERSION,
                sequence: self.sequence,
                phase,
                status,
                detail,
            },
        )?;
        writeln!(self.file)?;
        self.file.flush()?;
        self.file.sync_data()?;
        Ok(())
    }
}

struct FactoryLock {
    path: PathBuf,
}

impl FactoryLock {
    fn acquire(state_dir: &Path, project: &Project) -> Result<Self> {
        let path = state_dir.join("factory.lock");
        for attempt in 0..2 {
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)
            {
                Ok(mut file) => {
                    writeln!(file, "{}", std::process::id())?;
                    file.sync_all()?;
                    return Ok(Self { path });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists && attempt == 0 => {
                    if lock_owner_is_alive(&path) {
                        return Err(SheprdError::Message(format!(
                            "another factory run is already active for {}",
                            project.path.display()
                        )));
                    }
                    std::fs::remove_file(&path)?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    return Err(SheprdError::Message(format!(
                        "another factory run is already active for {}",
                        project.path.display()
                    )));
                }
                Err(error) => return Err(error.into()),
            }
        }
        Err(SheprdError::Message(
            "could not acquire the factory run lock".into(),
        ))
    }
}

impl Drop for FactoryLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn lock_owner_is_alive(path: &Path) -> bool {
    let Ok(contents) = std::fs::read_to_string(path) else {
        return true;
    };
    let Ok(pid) = contents.trim().parse::<u32>() else {
        return true;
    };
    if pid == std::process::id() {
        return true;
    }
    Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "pid="])
        .output()
        .is_ok_and(|output| output.status.success() && !output.stdout.is_empty())
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| SheprdError::Message("invalid factory receipt path".into()))?;
    let temporary = parent.join(format!(".receipt.{}.tmp", std::process::id()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)?;
        serde_json::to_writer_pretty(&mut file, value)?;
        writeln!(file)?;
        file.sync_all()?;
        std::fs::rename(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marker(nonce: &str) -> EnvelopeMarker {
        EnvelopeMarker::from_nonce(nonce)
    }

    fn plan_json(nonce: &str) -> String {
        format!(
            "{{\"schema_version\":1,\"kind\":\"plan\",\"nonce\":\"{nonce}\",\"summary\":\"safe\",\"steps\":[{{\"id\":\"P1\",\"objective\":\"edit\",\"allow_paths\":[\"src\"]}}]}}"
        )
    }

    fn response(marker: &EnvelopeMarker, body: &str) -> String {
        format!("{}\n{body}\n{}", marker.start, marker.end)
    }

    #[test]
    fn parses_one_nonce_bound_typed_plan_after_prompt_echo() {
        let marker = marker("fresh");
        let request = FactoryRequest {
            task: "edit safely".into(),
            allow_paths: vec!["src".into()],
            checks: vec!["true".into()],
            check_timeout_seconds: 300,
        };
        let prompt = plan_prompt(&request, &marker);
        assert!(!prompt.contains(MARKER_START_PREFIX));
        let text = format!("{prompt}\n{}", response(&marker, &plan_json("fresh")));
        let plan: PlanEnvelope = parse_envelope(&text, "plan", &marker).expect("plan");
        assert_eq!(plan.steps[0].id, "P1");
    }

    #[test]
    fn rejects_wrong_envelope_kind() {
        let marker = marker("fresh");
        let text = response(
            &marker,
            "{\"schema_version\":1,\"kind\":\"review\",\"nonce\":\"fresh\"}",
        );
        let error = parse_envelope::<PlanEnvelope>(&text, "plan", &marker).expect_err("wrong kind");
        assert!(error.to_string().contains("kind must be plan"));
    }

    #[test]
    fn rejects_duplicate_or_nested_envelope_blocks() {
        let marker = marker("fresh");
        assert!(parse_envelope::<PlanEnvelope>("no envelope", "plan", &marker).is_err());
        let one = response(&marker, &plan_json("fresh"));
        let duplicate = format!("{one}\n{one}");
        assert!(parse_envelope::<PlanEnvelope>(&duplicate, "plan", &marker).is_err());
        let nested = response(
            &marker,
            &format!("{}{}{}", plan_json("fresh"), marker.start, marker.end),
        );
        assert!(parse_envelope::<PlanEnvelope>(&nested, "plan", &marker).is_err());
        let extra = format!("{one}\n{LEGACY_MARKER_LIKE_PREFIX}");
        assert!(parse_envelope::<PlanEnvelope>(&extra, "plan", &marker).is_err());
    }

    #[test]
    fn rejects_wrong_stale_nonce_and_correction_replay() {
        let current = marker("current");
        let wrong_field = response(&current, &plan_json("stale"));
        assert!(parse_envelope::<PlanEnvelope>(&wrong_field, "plan", &current).is_err());

        let stale = marker("stale");
        let replay = response(&stale, &plan_json("stale"));
        let error = parse_envelope::<PlanEnvelope>(&replay, "plan", &current)
            .expect_err("correction replay");
        assert!(error.to_string().contains("stale or mismatched"));
    }

    #[test]
    fn prompt_content_redacts_forged_markers() {
        let marker = marker("fresh");
        let request = FactoryRequest {
            task: format!("inspect {MARKER_START_PREFIX}forged>>>"),
            allow_paths: vec!["src".into()],
            checks: vec![format!("printf {LEGACY_MARKER_LIKE_PREFIX}")],
            check_timeout_seconds: 300,
        };
        let prompt = plan_prompt(&request, &marker);
        assert!(!prompt.contains(MARKER_LIKE_PREFIX));
        assert!(!prompt.contains(LEGACY_MARKER_LIKE_PREFIX));
        assert!(prompt.contains("[redacted Sheprd factory marker]"));
    }

    #[test]
    fn oversized_task_and_plan_prompts_are_rejected_by_the_shared_limit() {
        let marker = marker("fresh");
        let request = FactoryRequest {
            task: "t".repeat(MAX_AGENT_PROMPT_BYTES + 1),
            allow_paths: vec!["src".into()],
            checks: vec!["true".into()],
            check_timeout_seconds: 300,
        };
        let task_prompt = plan_prompt(&request, &marker);
        assert!(require_agent_prompt_size(&task_prompt).is_err());

        let request = FactoryRequest {
            task: "bounded".into(),
            allow_paths: vec!["src".into()],
            checks: vec!["true".into()],
            check_timeout_seconds: 300,
        };
        let plan = PlanEnvelope {
            schema_version: FACTORY_SCHEMA_VERSION,
            kind: "plan".into(),
            nonce: "plan-nonce".into(),
            summary: "p".repeat(MAX_AGENT_PROMPT_BYTES + 1),
            steps: vec![PlanStep {
                id: "P1".into(),
                objective: "bounded".into(),
                allow_paths: vec!["src".into()],
            }],
        };
        let plan_prompt = implementation_prompt(&request, &plan, 0, None, &marker).expect("prompt");
        assert!(require_agent_prompt_size(&plan_prompt).is_err());
    }

    #[test]
    fn public_run_rejects_an_empty_check_list() {
        let temp = assert_fs::TempDir::new().expect("temp");
        let project = Project {
            name: "test".into(),
            path: temp.path().into(),
        };
        let error = run(
            &project,
            &FlokConfig::default(),
            FactoryRequest {
                task: "bounded task".into(),
                allow_paths: vec!["README.md".into()],
                checks: Vec::new(),
                check_timeout_seconds: 300,
            },
        )
        .expect_err("checks are required");
        assert!(error
            .to_string()
            .contains("factory check commands must not be empty"));
    }

    #[test]
    fn allow_paths_reject_parent_escape() {
        let error = normalize_allow_paths(&["../outside".into()]).expect_err("escape");
        assert!(error.to_string().contains("escapes the repository"));
    }

    #[test]
    fn allow_paths_match_files_and_descendants_only() {
        let allowed = normalize_allow_paths(&["src".into(), "README.md".into()]).expect("paths");
        assert!(path_is_allowed(Path::new("src/factory.rs"), &allowed));
        assert!(path_is_allowed(Path::new("README.md"), &allowed));
        assert!(!path_is_allowed(Path::new("tests/cli.rs"), &allowed));
    }

    #[test]
    fn first_disallowed_path_fails_closed_on_normalization_error() {
        let allowed = normalize_allow_paths(&["src".into()]).expect("allowed");
        let error = first_disallowed_path(&["../outside".into()], &allowed)
            .expect_err("normalization must fail closed");
        assert!(error.to_string().contains("escapes the repository"));
    }

    fn init_test_repo(path: &Path) {
        std::fs::create_dir_all(path).expect("repo directory");
        for args in [
            &["init", "-q"][..],
            &["config", "user.email", "test@example.com"][..],
            &["config", "user.name", "Sheprd Test"][..],
        ] {
            assert!(Command::new("git")
                .args(args)
                .current_dir(path)
                .status()
                .expect("git")
                .success());
        }
        std::fs::write(path.join("README.md"), "seed\n").expect("seed");
        assert!(Command::new("git")
            .args(["add", "README.md"])
            .current_dir(path)
            .status()
            .expect("add")
            .success());
        assert!(Command::new("git")
            .args(["commit", "-q", "-m", "seed"])
            .current_dir(path)
            .status()
            .expect("commit")
            .success());
    }

    fn test_repo() -> assert_fs::TempDir {
        let temp = assert_fs::TempDir::new().expect("temp repo");
        init_test_repo(temp.path());
        temp
    }

    fn test_linked_worktree() -> (assert_fs::TempDir, PathBuf) {
        let temp = assert_fs::TempDir::new().expect("temp worktree root");
        let base = temp.path().join("base");
        let worker = temp.path().join("worker");
        init_test_repo(&base);
        assert!(Command::new("git")
            .args(["worktree", "add", "-q", "--detach"])
            .arg(&worker)
            .arg("HEAD")
            .current_dir(&base)
            .status()
            .expect("worktree add")
            .success());
        (temp, worker)
    }

    #[test]
    fn check_timeout_kills_the_process_group() {
        let repo = test_repo();
        let head = git_head(repo.path()).expect("head");
        let started = Instant::now();
        let result = run_check(
            repo.path(),
            &head,
            "sleep 30 & wait",
            Duration::from_millis(100),
        )
        .expect("check");
        assert!(result.timed_out);
        assert!(!result.success);
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn check_source_mutation_is_a_failure() {
        let repo = test_repo();
        let head = git_head(repo.path()).expect("head");
        let result = run_check(
            repo.path(),
            &head,
            "printf mutation >> README.md",
            Duration::from_secs(2),
        )
        .expect("check");
        assert!(result.mutated_source);
        assert!(!result.success);
    }

    #[test]
    fn check_git_admin_entry_replacement_is_a_failure_and_restores_cleanly() {
        let (_temp, worker) = test_linked_worktree();
        let head = git_head(&worker).expect("head");
        let result = run_check(
            &worker,
            &head,
            "cp .git .git.sheprd-backup && trap 'if [ -f .git.sheprd-backup ]; then mv -f .git.sheprd-backup .git; fi' EXIT HUP INT TERM && printf 'gitdir: /definitely/not/a/repository\\n' > .git && mv -f .git.sheprd-backup .git && trap - EXIT HUP INT TERM",
            Duration::from_secs(2),
        )
        .expect("check");
        assert!(result.mutated_source);
        assert!(!result.success);
        assert_eq!(git_head(&worker).expect("restored git entry"), head);
        assert!(std::fs::symlink_metadata(worker.join(".git"))
            .expect("git entry")
            .is_file());
        assert!(!worker.join(".git.sheprd-backup").exists());
    }

    #[test]
    fn review_source_snapshot_detects_same_path_content_changes() {
        let repo = test_repo();
        let head = git_head(repo.path()).expect("head");
        std::fs::write(repo.path().join("factory.txt"), "reviewed\n").expect("source");
        let snapshot = source_snapshot(repo.path(), &head).expect("snapshot");
        std::fs::write(repo.path().join("factory.txt"), "mutated!\n").expect("mutation");
        let error =
            require_source_snapshot_unchanged(repo.path(), &head, &snapshot, "Claude review")
                .expect_err("review mutation");
        assert!(error
            .to_string()
            .contains("Codex worker source changed during Claude review"));
    }

    #[test]
    fn oversized_untracked_review_file_is_rejected_before_reading() {
        let repo = test_repo();
        let head = git_head(repo.path()).expect("head");
        std::fs::write(
            repo.path().join("large.txt"),
            vec![b'x'; MAX_REVIEW_PATCH_BYTES + 1],
        )
        .expect("large file");
        let error = review_patch(repo.path(), &head, &["large.txt".into()])
            .expect_err("oversized review file");
        assert!(error.to_string().contains("before reading untracked file"));
    }

    #[test]
    fn untracked_review_reader_rejects_path_and_handle_metadata_changes() {
        let temp = assert_fs::TempDir::new().expect("temp");
        let path = temp.path().join("review.txt");
        std::fs::write(&path, "original").expect("original");
        let expected = file_metadata_snapshot(
            &std::fs::symlink_metadata(&path).expect("metadata"),
            "review path",
        )
        .expect("snapshot");
        std::fs::rename(&path, temp.path().join("original.txt")).expect("rename");
        std::fs::write(&path, "replaced").expect("replacement");
        let error = read_untracked_review_file(&path, "review.txt", &expected)
            .expect_err("path replacement");
        assert!(error
            .to_string()
            .contains("changed while factory state was being read"));

        let file = File::open(&path).expect("open");
        let before = file_metadata_snapshot(&file.metadata().expect("before"), "handle")
            .expect("before snapshot");
        let mut permissions = std::fs::metadata(&path).expect("permissions").permissions();
        permissions.set_mode(permissions.mode() ^ 0o100);
        std::fs::set_permissions(&path, permissions).expect("chmod");
        let after = file_metadata_snapshot(&file.metadata().expect("after"), "handle")
            .expect("after snapshot");
        assert!(require_same_metadata(&before, &after, "untracked review file").is_err());
    }

    #[test]
    fn factory_artifacts_use_private_permissions() {
        let temp = assert_fs::TempDir::new().expect("temp");
        let state = temp.path().join("state");
        create_private_dir(&state).expect("private dir");
        assert_eq!(
            std::fs::metadata(&state)
                .expect("dir metadata")
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        let trace_path = state.join("trace.jsonl");
        let mut trace = TraceWriter::create(&trace_path).expect("trace");
        trace.append("test", "ok", json!({})).expect("append");
        assert_eq!(
            std::fs::metadata(&trace_path)
                .expect("trace metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        let receipt_path = state.join("receipt.json");
        write_json_atomic(&receipt_path, &json!({ "ok": true })).expect("receipt");
        assert_eq!(
            std::fs::metadata(&receipt_path)
                .expect("receipt metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        let project = Project {
            name: "test".into(),
            path: temp.path().into(),
        };
        let lock = FactoryLock::acquire(&state, &project).expect("lock");
        assert_eq!(
            std::fs::metadata(state.join("factory.lock"))
                .expect("lock metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        drop(lock);
    }
}

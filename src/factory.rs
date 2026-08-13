use crate::config::FlokConfig;
use crate::error::{Result, SheprdError};
use crate::herdr::{self, FlokAgent};
use crate::project::Project;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const FACTORY_ENVELOPE_SCHEMA_VERSION: u32 = 1;
const FACTORY_RECEIPT_SCHEMA_VERSION: u32 = 2;
const FACTORY_STATS_SCHEMA_VERSION: u32 = 1;
const FACTORY_CASES_SCHEMA_VERSION: u32 = 1;
const MAX_RECEIPT_BYTES: u64 = 2 * 1024 * 1024;
const MARKER_START_PREFIX: &str = "<<<SHEPRD_FACTORY_JSON_START:";
const MARKER_END_PREFIX: &str = "<<<SHEPRD_FACTORY_JSON_END:";
const MARKER_LIKE_PREFIX: &str = "<<<SHEPRD_FACTORY";
const LEGACY_MARKER_LIKE_PREFIX: &str = "<<<END_SHEPRD_FACTORY";
const MAX_CORRECTION_TURNS: usize = 2;
const MAX_CAPTURE_BYTES: usize = 8 * 1024;
const MAX_REVIEW_PATCH_BYTES: usize = 48 * 1024;
const MAX_AGENT_PROMPT_BYTES: usize = 60 * 1024;
const MAX_SELECTED_SKILLS: usize = 3;
const MAX_GIT_ADMIN_ENTRY_BYTES: usize = 8 * 1024;
const MAX_IGNORED_ENTRIES: usize = 100_000;
const MAX_IGNORED_ENUM_BYTES: usize = 16 * 1024 * 1024;
const MAX_IGNORED_TOTAL_FILE_BYTES: u64 = 64 * 1024 * 1024 * 1024;
const CHECK_POLL_INTERVAL: Duration = Duration::from_millis(25);
const AGENT_RESPONSE_POLL_INTERVAL: Duration = Duration::from_millis(100);
const AGENT_RESPONSE_SETTLE_TIMEOUT: Duration = Duration::from_secs(2);
const AGENT_RESPONSE_TIMEOUT: Duration = Duration::from_secs(120);
const OPENCODE_AGENT_RESPONSE_TIMEOUT: Duration = Duration::from_secs(300);
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
    pub plan: PlanEnvelope,
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
#[serde(deny_unknown_fields)]
pub struct TaskReference {
    pub id: String,
    pub number: u64,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillSelectionMode {
    #[default]
    None,
    Router,
    Explicit,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SkillReference {
    pub name: String,
    pub version: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PlanEnvelope {
    pub schema_version: u32,
    pub kind: String,
    pub nonce: String,
    pub summary: String,
    #[serde(default)]
    pub task_reference: Option<TaskReference>,
    #[serde(default)]
    pub skill_selection_mode: SkillSelectionMode,
    #[serde(default)]
    pub selected_skills: Vec<SkillReference>,
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

#[derive(Clone, Debug, Deserialize, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CheckAttempt {
    pub implementation_turn: usize,
    pub results: Vec<CheckResult>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AcceptanceOutcome {
    Accepted,
    Rejected,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureStage {
    Preflight,
    Plan,
    Implementation,
    Checks,
    ClaudeReview,
    OpencodeReview,
    FinalValidation,
}

impl FailureStage {
    fn label(self) -> &'static str {
        match self {
            Self::Preflight => "preflight",
            Self::Plan => "plan",
            Self::Implementation => "implementation",
            Self::Checks => "checks",
            Self::ClaudeReview => "claude_review",
            Self::OpencodeReview => "opencode_review",
            Self::FinalValidation => "final_validation",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewOutcome {
    NotReached,
    Approved,
    Rejected,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReviewOutcomes {
    pub claude: ReviewOutcome,
    pub opencode: ReviewOutcome,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CostAvailability {
    Unavailable,
    Authoritative,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthoritativeCost {
    pub source: String,
    pub currency: String,
    pub amount_minor_units: u64,
    pub minor_unit_scale: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReceiptCost {
    pub availability: CostAvailability,
    pub authoritative: Option<AuthoritativeCost>,
}

impl ReceiptCost {
    fn unavailable() -> Self {
        Self {
            availability: CostAvailability::Unavailable,
            authoritative: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
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
    pub acceptance: AcceptanceOutcome,
    pub failure_stage: Option<FailureStage>,
    pub review_outcomes: ReviewOutcomes,
    pub started_at_unix_ms: u64,
    pub finished_at_unix_ms: u64,
    pub elapsed_ms: u64,
    pub implementation_turn_count: usize,
    pub check_attempt_count: usize,
    pub cost: ReceiptCost,
    pub failure: Option<String>,
    pub trace_path: String,
    pub receipt_path: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct RateMetric {
    pub numerator: u64,
    pub denominator: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageAvailability {
    Unavailable,
    Partial,
    Complete,
}

impl CoverageAvailability {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unavailable => "unavailable",
            Self::Partial => "partial",
            Self::Complete => "complete",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct RuntimeStats {
    pub availability: CoverageAvailability,
    pub covered_runs: u64,
    pub total_runs: u64,
    pub total_elapsed_ms: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CostTotal {
    pub currency: String,
    pub amount_minor_units: u64,
    pub minor_unit_scale: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct CostStats {
    pub availability: CoverageAvailability,
    pub authoritative_runs: u64,
    pub total_runs: u64,
    pub totals: Vec<CostTotal>,
}

#[derive(Clone, Debug, Serialize)]
pub struct FactoryStats {
    pub schema_version: u32,
    pub project: String,
    pub total_runs: u64,
    pub incomplete_runs: u64,
    pub accepted_runs: u64,
    pub rejected_runs: u64,
    pub acceptance: RateMetric,
    pub corrections: RateMetric,
    pub check_attempts: u64,
    pub failure_stages: BTreeMap<String, u64>,
    pub runtime: RuntimeStats,
    pub cost: CostStats,
}

#[derive(Clone, Debug, Serialize)]
pub struct FactoryCase {
    pub run_id: String,
    pub task: String,
    pub task_reference: Option<TaskReference>,
    pub skill_selection_mode: SkillSelectionMode,
    pub selected_skills: Vec<SkillReference>,
    pub allow_paths: Vec<String>,
    pub check_commands: Vec<String>,
    pub changed_paths: Vec<String>,
    pub accepted: bool,
    pub failure_stage: Option<String>,
    pub review_outcomes: ReviewOutcomes,
    pub elapsed_ms: Option<u64>,
    pub implementation_turn_count: usize,
    pub check_attempt_count: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct FactoryCases {
    pub schema_version: u32,
    pub project: String,
    pub total_completed_runs: u64,
    pub incomplete_runs: u64,
    pub limit: usize,
    pub cases: Vec<FactoryCase>,
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
    ignored: Option<IgnoredStateSnapshot>,
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
    uid: u32,
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
    let started = Instant::now();
    let started_at_unix_ms = unix_time_ms()?;
    let allowed = normalize_allow_paths(&request.allow_paths)?;
    validate_plan(&request.plan, &allowed)?;
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
        schema_version: FACTORY_RECEIPT_SCHEMA_VERSION,
        run_id: run_id.clone(),
        project: project.name.clone(),
        task: request.task.clone(),
        allow_paths: request.allow_paths.clone(),
        check_commands: request.checks.clone(),
        check_timeout_seconds: request.check_timeout_seconds,
        workspace_id: None,
        plan: Some(request.plan.clone()),
        implementations: Vec::new(),
        check_attempts: Vec::new(),
        claude_review: None,
        opencode_review: None,
        changed_paths: Vec::new(),
        base_unchanged: true,
        worker_head_unchanged: true,
        accepted: false,
        acceptance: AcceptanceOutcome::Rejected,
        failure_stage: Some(FailureStage::Preflight),
        review_outcomes: ReviewOutcomes {
            claude: ReviewOutcome::NotReached,
            opencode: ReviewOutcome::NotReached,
        },
        started_at_unix_ms,
        finished_at_unix_ms: started_at_unix_ms,
        elapsed_ms: 0,
        implementation_turn_count: 0,
        check_attempt_count: 0,
        cost: ReceiptCost::unavailable(),
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
            "orchestrator": "pi",
            "task_reference": &request.plan.task_reference,
            "skill_selection_mode": request.plan.skill_selection_mode,
            "selected_skills": &request.plan.selected_skills,
            "allow_paths": request.allow_paths,
            "checks": request.checks,
        }),
    )?;

    if let Err(error) = execute_factory(&mut context, flok_config, &mut receipt, &mut trace) {
        receipt.failure = Some(error.to_string());
        trace.append("run", "failed", json!({ "error": error.to_string() }))?;
    }

    if receipt.failure.is_none() {
        receipt.failure_stage = Some(FailureStage::FinalValidation);
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
        receipt.failure =
            Some("implementation worker HEAD changed; factory runs must not commit".into());
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
        receipt.failure_stage = Some(
            if receipt
                .claude_review
                .as_ref()
                .is_none_or(|review| !review.approved)
            {
                FailureStage::ClaudeReview
            } else {
                FailureStage::OpencodeReview
            },
        );
        receipt.failure = Some("both independent reviewers must approve acceptance".into());
    }
    receipt.accepted = receipt.failure.is_none()
        && receipt.base_unchanged
        && receipt.worker_head_unchanged
        && checks_pass
        && reviews_approve;
    receipt.acceptance = if receipt.accepted {
        AcceptanceOutcome::Accepted
    } else {
        AcceptanceOutcome::Rejected
    };
    receipt.failure_stage = (!receipt.accepted).then_some(
        receipt
            .failure_stage
            .unwrap_or(FailureStage::FinalValidation),
    );
    receipt.review_outcomes = review_outcomes(&receipt);
    receipt.implementation_turn_count = receipt.implementations.len();
    receipt.check_attempt_count = receipt.check_attempts.len();
    receipt.finished_at_unix_ms = unix_time_ms()?.max(receipt.started_at_unix_ms);
    receipt.elapsed_ms = u64::try_from(started.elapsed().as_millis())
        .map_err(|_| SheprdError::Message("factory elapsed runtime overflow".into()))?;

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

fn review_outcomes(receipt: &FactoryReceipt) -> ReviewOutcomes {
    ReviewOutcomes {
        claude: review_outcome(receipt.claude_review.as_ref()),
        opencode: review_outcome(receipt.opencode_review.as_ref()),
    }
}

fn review_outcome(review: Option<&ReviewEnvelope>) -> ReviewOutcome {
    match review {
        Some(review) if review.approved => ReviewOutcome::Approved,
        Some(_) => ReviewOutcome::Rejected,
        None => ReviewOutcome::NotReached,
    }
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

    let codex = agent(&flok.agents, "codex")?;
    let claude = agent(&flok.agents, "claude")?;
    let opencode = agent(&flok.agents, "opencode")?;
    let worker_path = PathBuf::from(&codex.cwd);
    let claude_before = git_snapshot(Path::new(&claude.cwd))?;
    let opencode_before = git_snapshot(Path::new(&opencode.cwd))?;
    let worker_head = git_head(&worker_path)?;
    if worker_head != context.base_before.head {
        return Err(SheprdError::Message(format!(
            "implementation worker HEAD is stale: worker={worker_head} base={}",
            context.base_before.head
        )));
    }
    if !changed_paths(&worker_path, &worker_head)?.is_empty() {
        return Err(SheprdError::Message(
            "implementation worker checkout must be clean before a factory run".into(),
        ));
    }
    context.worker_path = Some(worker_path.clone());
    context.worker_head = Some(worker_head.clone());
    verify_integrity(context)?;

    let plan = context.request.plan.clone();
    trace.append("plan", "provided_by_pi", serde_json::to_value(&plan)?)?;
    verify_integrity(context)?;

    receipt.failure_stage = Some(FailureStage::Implementation);
    let implementation_marker = EnvelopeMarker::fresh()?;
    let initial_prompt =
        implementation_prompt(context.request, &plan, 0, None, &implementation_marker)?;
    let implementation_ignored = ignored_state_snapshot(&worker_path)?;
    let implementation_result = run_agent_phase(
        codex,
        &initial_prompt,
        "implementation",
        &implementation_marker,
        trace,
    );
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
        receipt.failure_stage = Some(FailureStage::Checks);
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
        receipt.failure_stage = Some(FailureStage::Implementation);
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
            &correction_marker,
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

    receipt.failure_stage = Some(FailureStage::FinalValidation);
    let review_source = source_snapshot(&worker_path, &worker_head, true)?;
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

    receipt.failure_stage = Some(FailureStage::ClaudeReview);
    let claude_marker = EnvelopeMarker::fresh()?;
    let claude_prompt = review_prompt(
        "claude",
        "intent review: verify the implementation matches the typed plan and task",
        &worker_path,
        context.request,
        &plan,
        final_checks,
        &actual_paths,
        &patch,
        &claude_marker,
    )?;
    let claude_review = run_review_phase(
        claude,
        &claude_prompt,
        "claude_review",
        &claude_marker,
        trace,
    );
    if git_snapshot(Path::new(&claude.cwd))? != claude_before {
        return Err(SheprdError::Message(
            "Claude review modified its checkout".into(),
        ));
    }
    let claude_review = claude_review?;
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

    receipt.failure_stage = Some(FailureStage::OpencodeReview);
    let opencode_marker = EnvelopeMarker::fresh()?;
    let opencode_prompt = review_prompt(
        "opencode",
        "adversarial review: search for correctness, safety, scope, and test gaps",
        &worker_path,
        context.request,
        &plan,
        final_checks,
        &actual_paths,
        &patch,
        &opencode_marker,
    )?;
    let opencode_review = run_review_phase(
        opencode,
        &opencode_prompt,
        "opencode_review",
        &opencode_marker,
        trace,
    );
    if git_snapshot(Path::new(&opencode.cwd))? != opencode_before {
        return Err(SheprdError::Message(
            "OpenCode review modified its checkout".into(),
        ));
    }
    let opencode_review = opencode_review?;
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
    receipt.failure_stage = Some(FailureStage::FinalValidation);
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
    marker: &EnvelopeMarker,
    trace: &mut TraceWriter,
) -> Result<String> {
    require_agent_prompt_size(prompt)?;
    trace.append(
        phase,
        "prompted",
        json!({ "agent": agent.name, "kind": agent.kind }),
    )?;
    let prompt_outcome = herdr::prompt_agent(&agent.name, prompt)?;
    if prompt_outcome == herdr::PromptOutcome::WaitRecovered {
        trace.append(
            phase,
            "prompt_wait_recovered",
            json!({ "agent": agent.name, "kind": agent.kind }),
        )?;
    }
    let deadline = Instant::now() + agent_response_timeout(&agent.kind);
    let recovery_at = Instant::now() + AGENT_RESPONSE_SETTLE_TIMEOUT;
    let mut recovery_prompted = false;
    let text = loop {
        let text = normalize_agent_output(&herdr::read_agent_response(&agent.name, &agent.kind)?);
        if (text.contains(&marker.start) && text.contains(&marker.end))
            || (text.contains(MARKER_START_PREFIX) && text.contains(MARKER_END_PREFIX))
        {
            break text;
        }
        let now = Instant::now();
        if !recovery_prompted && now >= recovery_at {
            let recovery_prompt = format!(
                "Factory {phase} envelope recovery. The previous typed envelope is not visible to the controller. Do not redo the work. Re-emit only the existing {phase} envelope.\n{}",
                marker.instructions()
            );
            require_agent_prompt_size(&recovery_prompt)?;
            trace.append(
                phase,
                "recovery_prompted",
                json!({ "agent": agent.name, "kind": agent.kind }),
            )?;
            let prompt_outcome = herdr::prompt_agent(&agent.name, &recovery_prompt)?;
            if prompt_outcome == herdr::PromptOutcome::WaitRecovered {
                trace.append(
                    phase,
                    "prompt_wait_recovered",
                    json!({ "agent": agent.name, "kind": agent.kind }),
                )?;
            }
            recovery_prompted = true;
            continue;
        }
        if now >= deadline {
            return Err(SheprdError::Message(format!(
                "timed out waiting for a complete {phase} factory envelope from {}",
                agent.name
            )));
        }
        std::thread::sleep(AGENT_RESPONSE_POLL_INTERVAL);
    };
    trace.append(
        phase,
        "responded",
        json!({ "agent": agent.name, "bytes": text.len() }),
    )?;
    Ok(text)
}

fn agent_response_timeout(kind: &str) -> Duration {
    if kind == "opencode" {
        OPENCODE_AGENT_RESPONSE_TIMEOUT
    } else {
        AGENT_RESPONSE_TIMEOUT
    }
}

fn run_review_phase(
    agent: &FlokAgent,
    prompt: &str,
    phase: &str,
    marker: &EnvelopeMarker,
    trace: &mut TraceWriter,
) -> Result<ReviewEnvelope> {
    let text = run_agent_phase(agent, prompt, phase, marker, trace)?;
    match parse_envelope(&text, "review", marker) {
        Ok(review) => Ok(review),
        Err(error) => {
            let correction_marker = EnvelopeMarker::fresh()?;
            let correction_prompt = format!(
                "Factory review envelope correction. Your previous review envelope was invalid JSON: {}. Do not repeat the review. Preserve the existing verdict and findings. Re-emit only a corrected review envelope. Required JSON fields: {{\"schema_version\":1,\"kind\":\"review\",\"nonce\":\"{}\",\"reviewer\":\"{}\",\"approved\":false,\"summary\":\"...\",\"findings\":[\"...\"]}}\n{}",
                escape_prompt_content(&error.to_string()),
                correction_marker.nonce,
                agent.kind,
                correction_marker.instructions()
            );
            require_agent_prompt_size(&correction_prompt)?;
            trace.append(
                phase,
                "envelope_correction_prompted",
                json!({ "agent": agent.name, "kind": agent.kind, "error": error.to_string() }),
            )?;
            let corrected_text =
                run_agent_phase(agent, &correction_prompt, phase, &correction_marker, trace)?;
            parse_envelope(&corrected_text, "review", &correction_marker)
        }
    }
}

fn normalize_agent_output(text: &str) -> String {
    // Agent TUIs can hard-wrap terminal rows even when Herdr requests an
    // unwrapped snapshot. Factory envelopes are minified, so joining rows and
    // removing the TUI response indent reconstructs the exact
    // marker and JSON tokens. Human-facing summary whitespace is non-semantic.
    text.lines()
        .map(|line| line.trim_start_matches(' '))
        .collect()
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
        format!(
            "Envelope nonce: {}\nReturn no prose. Final response must be:\n{}\n<one minified valid JSON object>\n{}\nCopy both markers exactly. JSON strings must not contain unescaped quotes.",
            self.nonce, self.start, self.end
        )
    }
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
    worker_path: &Path,
    request: &FactoryRequest,
    plan: &PlanEnvelope,
    checks: &[CheckResult],
    changed_paths: &[String],
    patch: &str,
    marker: &EnvelopeMarker,
) -> Result<String> {
    Ok(format!(
        "Factory {purpose}. Review only; do not edit, delegate, commit, merge, or push. Fail closed on ambiguity. Inspect the supplied patch and check results once. Do not repeat analysis. Do not use tools unless one read-only inspection is necessary. Return the typed review envelope immediately after the review. Keep the summary and findings concise. A non-blocking coverage gap is not a reason to reject a correct change.\nReview target checkout: {}\nYour current working directory is an intentionally clean reviewer baseline and does not contain the implementation. Run any read-only repository inspection against the review target checkout above; never reject merely because your own cwd is clean.\nTask: {}\nAllowed paths: {}\nPlan: {}\nActual changed paths: {}\nRust check results: {}\nPatch:\n{}\nRequired JSON fields: {{\"schema_version\":1,\"kind\":\"review\",\"nonce\":\"{}\",\"reviewer\":\"{reviewer}\",\"approved\":false,\"summary\":\"...\",\"findings\":[\"...\"]}}\n{}",
        escape_prompt_content(&worker_path.display().to_string()),
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
    let value = redact_marker_tokens(value, MARKER_LIKE_PREFIX);
    redact_marker_tokens(&value, LEGACY_MARKER_LIKE_PREFIX)
}

fn redact_marker_tokens(value: &str, prefix: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut remaining = value;
    while let Some(start) = remaining.find(prefix) {
        output.push_str(&remaining[..start]);
        output.push_str("[redacted Sheprd factory marker]");
        remaining = &remaining[start + prefix.len()..];
        if let Some(end) = remaining.find(">>>") {
            remaining = &remaining[end + 3..];
        } else {
            remaining = "";
        }
    }
    output.push_str(remaining);
    output
}

fn parse_envelope<T: DeserializeOwned>(
    text: &str,
    expected_kind: &str,
    marker: &EnvelopeMarker,
) -> Result<T> {
    let start_count = text.matches(&marker.start).count();
    let end_count = text.matches(&marker.end).count();
    if start_count == 0
        && end_count == 0
        && (text.contains(MARKER_START_PREFIX) || text.contains(MARKER_END_PREFIX))
    {
        return Err(SheprdError::Message(
            "agent response has a stale or mismatched envelope nonce".into(),
        ));
    }
    let prompt_echo = text.find(&marker.start).is_some_and(|start| {
        text[..start]
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>()
            .contains("Finalresponsemustbe:")
    });
    let expected_count = if prompt_echo { 2 } else { 1 };
    if start_count != expected_count || end_count != expected_count {
        return Err(SheprdError::Message(
            "agent response must contain exactly one factory envelope pair".into(),
        ));
    }
    let start = text.rfind(&marker.start).ok_or_else(|| {
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
    if body.contains(MARKER_LIKE_PREFIX)
        || body.contains(LEGACY_MARKER_LIKE_PREFIX)
        || before.contains(LEGACY_MARKER_LIKE_PREFIX)
        || after.contains(LEGACY_MARKER_LIKE_PREFIX)
    {
        return Err(SheprdError::Message(
            "agent response contains nested or extra factory markers".into(),
        ));
    }
    let value: Value = serde_json::from_str(body.trim()).map_err(|error| {
        SheprdError::Message(format!("agent envelope is not valid JSON: {error}"))
    })?;
    if value.get("schema_version").and_then(Value::as_u64)
        != Some(u64::from(FACTORY_ENVELOPE_SCHEMA_VERSION))
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
    if plan.schema_version != FACTORY_ENVELOPE_SCHEMA_VERSION
        || plan.kind != "plan"
        || plan.nonce.trim().is_empty()
    {
        return Err(SheprdError::Message(
            "typed plan must be a schema-1 plan envelope with a nonce".into(),
        ));
    }
    if plan.summary.trim().is_empty() || plan.steps.is_empty() {
        return Err(SheprdError::Message(
            "typed plan must include a summary and at least one step".into(),
        ));
    }
    if let Some(task) = &plan.task_reference {
        if task.id.trim().is_empty() || task.number == 0 {
            return Err(SheprdError::Message(
                "typed plan task reference requires an id and positive number".into(),
            ));
        }
    }
    if plan.selected_skills.len() > MAX_SELECTED_SKILLS {
        return Err(SheprdError::Message(format!(
            "typed plan may select at most {MAX_SELECTED_SKILLS} skills"
        )));
    }
    if plan.skill_selection_mode == SkillSelectionMode::None && !plan.selected_skills.is_empty() {
        return Err(SheprdError::Message(
            "typed plan cannot select skills when selection mode is none".into(),
        ));
    }
    if plan.skill_selection_mode == SkillSelectionMode::Explicit && plan.selected_skills.is_empty()
    {
        return Err(SheprdError::Message(
            "typed plan explicit skill selection requires at least one skill".into(),
        ));
    }
    let mut skill_names = BTreeSet::new();
    for skill in &plan.selected_skills {
        if !valid_skill_name(&skill.name) || !valid_skill_version(&skill.version) {
            return Err(SheprdError::Message(
                "typed plan skills require valid names and versions".into(),
            ));
        }
        if !skill_names.insert(skill.name.as_str()) {
            return Err(SheprdError::Message(format!(
                "typed plan contains duplicate skill: {}",
                skill.name
            )));
        }
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

fn valid_skill_name(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 64
        && bytes[0].is_ascii_lowercase()
        && bytes[bytes.len() - 1].is_ascii_alphanumeric()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
        && !value.contains("--")
}

fn valid_skill_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+' | b'_'))
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
    if &source_snapshot(cwd, initial_head, expected.ignored.is_some())? != expected {
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
    let before = source_snapshot(cwd, initial_head, false)?;
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
    match source_snapshot(cwd, initial_head, before.ignored.is_some()) {
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
        uid: metadata.uid(),
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

fn source_snapshot(
    cwd: &Path,
    initial_head: &str,
    include_ignored: bool,
) -> Result<SourceSnapshot> {
    let git_admin = git_admin_snapshot(cwd)?;
    let ignored = include_ignored
        .then(|| ignored_state_snapshot(cwd))
        .transpose()?;
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
        ignored,
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct LegacyFactoryReceipt {
    schema_version: u32,
    run_id: String,
    project: String,
    task: String,
    allow_paths: Vec<String>,
    check_commands: Vec<String>,
    check_timeout_seconds: u64,
    workspace_id: Option<String>,
    plan: Option<PlanEnvelope>,
    implementations: Vec<ImplementationEnvelope>,
    check_attempts: Vec<CheckAttempt>,
    claude_review: Option<ReviewEnvelope>,
    opencode_review: Option<ReviewEnvelope>,
    changed_paths: Vec<String>,
    base_unchanged: bool,
    worker_head_unchanged: bool,
    accepted: bool,
    failure: Option<String>,
    trace_path: String,
    receipt_path: String,
}

struct ObservedRun {
    run_id: String,
    task: String,
    task_reference: Option<TaskReference>,
    skill_selection_mode: SkillSelectionMode,
    selected_skills: Vec<SkillReference>,
    allow_paths: Vec<String>,
    check_commands: Vec<String>,
    changed_paths: Vec<String>,
    accepted: bool,
    review_outcomes: ReviewOutcomes,
    implementation_turn_count: usize,
    check_attempt_count: usize,
    failure_stage: Option<String>,
    elapsed_ms: Option<u64>,
    cost: Option<AuthoritativeCost>,
}

pub fn stats(project: &Project) -> Result<FactoryStats> {
    let project_state = factory_project_state_path(project)?;
    let (runs, incomplete_runs) = read_stable_receipts(&project_state, project)?;
    aggregate_stats(project, runs, incomplete_runs)
}

pub fn cases(project: &Project, limit: usize) -> Result<FactoryCases> {
    if limit == 0 {
        return Err(SheprdError::Message(
            "factory case limit must be greater than zero".into(),
        ));
    }
    let project_state = factory_project_state_path(project)?;
    let (mut runs, incomplete_runs) = read_stable_receipts(&project_state, project)?;
    let total_completed_runs = u64::try_from(runs.len())
        .map_err(|_| SheprdError::Message("factory run count overflow".into()))?;
    runs.sort_by(|left, right| right.run_id.cmp(&left.run_id));
    let cases = runs
        .into_iter()
        .take(limit)
        .map(|run| FactoryCase {
            run_id: run.run_id,
            task: run.task,
            task_reference: run.task_reference,
            skill_selection_mode: run.skill_selection_mode,
            selected_skills: run.selected_skills,
            allow_paths: run.allow_paths,
            check_commands: run.check_commands,
            changed_paths: run.changed_paths,
            accepted: run.accepted,
            failure_stage: run.failure_stage,
            review_outcomes: run.review_outcomes,
            elapsed_ms: run.elapsed_ms,
            implementation_turn_count: run.implementation_turn_count,
            check_attempt_count: run.check_attempt_count,
        })
        .collect();
    Ok(FactoryCases {
        schema_version: FACTORY_CASES_SCHEMA_VERSION,
        project: project.name.clone(),
        total_completed_runs,
        incomplete_runs,
        limit,
        cases,
    })
}

fn aggregate_stats(
    project: &Project,
    runs: Vec<ObservedRun>,
    incomplete_runs: u64,
) -> Result<FactoryStats> {
    let total_runs = u64::try_from(runs.len())
        .map_err(|_| SheprdError::Message("factory run count overflow".into()))?;
    let mut accepted_runs = 0_u64;
    let mut correction_numerator = 0_u64;
    let mut correction_denominator = 0_u64;
    let mut check_attempts = 0_u64;
    let mut failure_stages = BTreeMap::new();
    let mut runtime_runs = 0_u64;
    let mut total_elapsed_ms = 0_u64;
    let mut authoritative_runs = 0_u64;
    let mut cost_totals: BTreeMap<(String, u32), u64> = BTreeMap::new();

    for run in runs {
        if run.accepted {
            accepted_runs = checked_increment(accepted_runs, "accepted run count")?;
        } else if let Some(stage) = run.failure_stage {
            checked_map_increment(&mut failure_stages, stage, "failure-stage count")?;
        }
        if run.implementation_turn_count > 0 {
            correction_denominator =
                checked_increment(correction_denominator, "correction denominator")?;
            if run.implementation_turn_count > 1 {
                correction_numerator =
                    checked_increment(correction_numerator, "correction numerator")?;
            }
        }
        check_attempts = check_attempts
            .checked_add(
                u64::try_from(run.check_attempt_count).map_err(|_| {
                    SheprdError::Message("factory check-attempt count overflow".into())
                })?,
            )
            .ok_or_else(|| SheprdError::Message("factory check-attempt total overflow".into()))?;
        if let Some(elapsed_ms) = run.elapsed_ms {
            runtime_runs = checked_increment(runtime_runs, "runtime coverage")?;
            total_elapsed_ms = total_elapsed_ms
                .checked_add(elapsed_ms)
                .ok_or_else(|| SheprdError::Message("factory runtime total overflow".into()))?;
        }
        if let Some(cost) = run.cost {
            authoritative_runs = checked_increment(authoritative_runs, "cost coverage")?;
            let total = cost_totals
                .entry((cost.currency, cost.minor_unit_scale))
                .or_default();
            *total = total
                .checked_add(cost.amount_minor_units)
                .ok_or_else(|| SheprdError::Message("factory cost total overflow".into()))?;
        }
    }

    let rejected_runs = total_runs
        .checked_sub(accepted_runs)
        .ok_or_else(|| SheprdError::Message("factory acceptance counts are inconsistent".into()))?;
    let totals = cost_totals
        .into_iter()
        .map(
            |((currency, minor_unit_scale), amount_minor_units)| CostTotal {
                currency,
                amount_minor_units,
                minor_unit_scale,
            },
        )
        .collect();
    Ok(FactoryStats {
        schema_version: FACTORY_STATS_SCHEMA_VERSION,
        project: project.name.clone(),
        total_runs,
        incomplete_runs,
        accepted_runs,
        rejected_runs,
        acceptance: RateMetric {
            numerator: accepted_runs,
            denominator: total_runs,
        },
        corrections: RateMetric {
            numerator: correction_numerator,
            denominator: correction_denominator,
        },
        check_attempts,
        failure_stages,
        runtime: RuntimeStats {
            availability: coverage(runtime_runs, total_runs),
            covered_runs: runtime_runs,
            total_runs,
            total_elapsed_ms,
        },
        cost: CostStats {
            availability: coverage(authoritative_runs, total_runs),
            authoritative_runs,
            total_runs,
            totals,
        },
    })
}

fn coverage(covered: u64, total: u64) -> CoverageAvailability {
    if total == 0 || covered == 0 {
        CoverageAvailability::Unavailable
    } else if covered == total {
        CoverageAvailability::Complete
    } else {
        CoverageAvailability::Partial
    }
}

fn checked_increment(value: u64, description: &str) -> Result<u64> {
    value
        .checked_add(1)
        .ok_or_else(|| SheprdError::Message(format!("factory {description} overflow")))
}

fn checked_map_increment(
    values: &mut BTreeMap<String, u64>,
    key: String,
    description: &str,
) -> Result<()> {
    let value = values.entry(key).or_default();
    *value = checked_increment(*value, description)?;
    Ok(())
}

fn read_stable_receipts(
    project_state: &Path,
    project: &Project,
) -> Result<(Vec<ObservedRun>, u64)> {
    let factory_root = project_state
        .parent()
        .ok_or_else(|| SheprdError::Message("invalid factory state path".into()))?;
    let state_root = factory_root
        .parent()
        .ok_or_else(|| SheprdError::Message("invalid factory state root".into()))?;
    let Some(state_before) = state_root_snapshot(state_root)? else {
        return Ok((Vec::new(), 0));
    };
    let owner_uid = state_before.uid;
    let Some(factory_before) = private_directory_snapshot(factory_root, Some(owner_uid))? else {
        let state_after = state_root_snapshot(state_root)?.ok_or_else(|| {
            SheprdError::Message("factory state changed while statistics were being read".into())
        })?;
        require_same_metadata(&state_before, &state_after, "Sheprd state directory")?;
        return Ok((Vec::new(), 0));
    };
    let Some(project_before) = private_directory_snapshot(project_state, Some(owner_uid))? else {
        let factory_after = private_directory_snapshot(factory_root, None)?.ok_or_else(|| {
            SheprdError::Message("factory state changed while statistics were being read".into())
        })?;
        require_same_metadata(&factory_before, &factory_after, "factory state directory")?;
        let state_after = state_root_snapshot(state_root)?.ok_or_else(|| {
            SheprdError::Message("Sheprd state changed while statistics were being read".into())
        })?;
        require_same_metadata(&state_before, &state_after, "Sheprd state directory")?;
        return Ok((Vec::new(), 0));
    };

    let entries = stable_directory_entries(project_state, &project_before)?;
    let mut runs = Vec::new();
    let mut incomplete_runs = 0_u64;
    for (name, path) in entries {
        if name == "factory.lock" {
            require_stale_factory_lock(&path, owner_uid)?;
            continue;
        }
        if name.starts_with(".receipt.") {
            return Err(SheprdError::Message(
                "factory state is incomplete; statistics require a stable snapshot".into(),
            ));
        }
        let run_before = private_directory_snapshot(&path, Some(owner_uid))?.ok_or_else(|| {
            SheprdError::Message("factory run disappeared while statistics were being read".into())
        })?;
        let run_entries = stable_directory_entries(&path, &run_before)?;
        let mut receipt_path = None;
        let mut trace_path = None;
        for (entry_name, entry_path) in run_entries {
            match entry_name.as_str() {
                "receipt.json" => receipt_path = Some(entry_path),
                "trace.jsonl" => trace_path = Some(entry_path),
                _ => {
                    return Err(SheprdError::Message(format!(
                        "unexpected factory run artifact: {entry_name}"
                    )))
                }
            }
        }
        let receipt_path = match receipt_path {
            None => {
                let trace_before = trace_path
                    .as_ref()
                    .map(|path| private_file_snapshot(path, owner_uid, "factory trace"))
                    .transpose()?;
                let run_after =
                    private_directory_snapshot(&path, Some(owner_uid))?.ok_or_else(|| {
                        SheprdError::Message(
                            "factory run disappeared while statistics were being read".into(),
                        )
                    })?;
                require_same_metadata(&run_before, &run_after, "factory run directory")?;
                if let (Some(trace_path), Some(trace_before)) = (trace_path, trace_before) {
                    let trace_after =
                        private_file_snapshot(&trace_path, owner_uid, "factory trace")?;
                    require_same_metadata(&trace_before, &trace_after, "factory trace")?;
                }
                incomplete_runs = checked_increment(incomplete_runs, "incomplete run count")?;
                continue;
            }
            Some(receipt_path) => receipt_path,
        };
        let trace_path = trace_path
            .ok_or_else(|| SheprdError::Message(format!("factory run {name} has no trace")))?;
        let trace_before = private_file_snapshot(&trace_path, owner_uid, "factory trace")?;
        let bytes = read_private_file(
            &receipt_path,
            owner_uid,
            MAX_RECEIPT_BYTES,
            "factory receipt",
        )?;
        let trace_after = private_file_snapshot(&trace_path, owner_uid, "factory trace")?;
        require_same_metadata(&trace_before, &trace_after, "factory trace")?;
        let run_after = private_directory_snapshot(&path, Some(owner_uid))?.ok_or_else(|| {
            SheprdError::Message("factory run disappeared while statistics were being read".into())
        })?;
        require_same_metadata(&run_before, &run_after, "factory run directory")?;
        runs.push(parse_observed_receipt(
            &bytes,
            project,
            &name,
            &receipt_path,
            &trace_path,
        )?);
    }
    let project_after =
        private_directory_snapshot(project_state, Some(owner_uid))?.ok_or_else(|| {
            SheprdError::Message(
                "factory state disappeared while statistics were being read".into(),
            )
        })?;
    require_same_metadata(&project_before, &project_after, "factory project directory")?;
    let factory_after = private_directory_snapshot(factory_root, None)?.ok_or_else(|| {
        SheprdError::Message("factory state disappeared while statistics were being read".into())
    })?;
    require_same_metadata(&factory_before, &factory_after, "factory state directory")?;
    let state_after = state_root_snapshot(state_root)?.ok_or_else(|| {
        SheprdError::Message("Sheprd state disappeared while statistics were being read".into())
    })?;
    require_same_metadata(&state_before, &state_after, "Sheprd state directory")?;
    Ok((runs, incomplete_runs))
}

fn stable_directory_entries(
    path: &Path,
    before: &FileMetadataSnapshot,
) -> Result<Vec<(String, PathBuf)>> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| SheprdError::Message("factory state contains a non-UTF-8 entry".into()))?;
        entries.push((name, entry.path()));
    }
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    let after =
        file_metadata_snapshot(&std::fs::symlink_metadata(path)?, "factory state directory")?;
    require_same_metadata(before, &after, "factory state directory")?;
    Ok(entries)
}

fn private_directory_snapshot(
    path: &Path,
    expected_uid: Option<u32>,
) -> Result<Option<FileMetadataSnapshot>> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let snapshot = file_metadata_snapshot(&metadata, "factory state directory")?;
    if snapshot.kind != FileKind::Directory || snapshot.mode & 0o777 != 0o700 {
        return Err(SheprdError::Message(format!(
            "factory state directory must be an owner-only 0700 directory: {}",
            path.display()
        )));
    }
    if expected_uid.is_some_and(|uid| snapshot.uid != uid) {
        return Err(SheprdError::Message(format!(
            "factory state ownership is inconsistent: {}",
            path.display()
        )));
    }
    Ok(Some(snapshot))
}

fn state_root_snapshot(path: &Path) -> Result<Option<FileMetadataSnapshot>> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let snapshot = file_metadata_snapshot(&metadata, "Sheprd state directory")?;
    if snapshot.kind != FileKind::Directory {
        return Err(SheprdError::Message(format!(
            "Sheprd state root must be a real directory: {}",
            path.display()
        )));
    }
    Ok(Some(snapshot))
}

fn private_file_snapshot(
    path: &Path,
    owner_uid: u32,
    description: &str,
) -> Result<FileMetadataSnapshot> {
    let snapshot = file_metadata_snapshot(&std::fs::symlink_metadata(path)?, description)?;
    if snapshot.kind != FileKind::Regular
        || snapshot.mode & 0o777 != 0o600
        || snapshot.uid != owner_uid
    {
        return Err(SheprdError::Message(format!(
            "{description} must be an owner-only 0600 regular file: {}",
            path.display()
        )));
    }
    Ok(snapshot)
}

fn read_private_file(
    path: &Path,
    owner_uid: u32,
    limit: u64,
    description: &str,
) -> Result<Vec<u8>> {
    let expected = private_file_snapshot(path, owner_uid, description)?;
    if expected.len > limit {
        return Err(SheprdError::Message(format!(
            "{description} exceeds the {limit} byte limit"
        )));
    }
    let mut file = File::open(path)?;
    let opened = file_metadata_snapshot(&file.metadata()?, &format!("{description} handle"))?;
    require_same_metadata(&expected, &opened, description)?;
    let mut bytes = Vec::new();
    (&mut file)
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if u64::try_from(bytes.len()).ok() != Some(expected.len) {
        return Err(SheprdError::Message(format!(
            "{description} changed while statistics were being read"
        )));
    }
    let handle_after = file_metadata_snapshot(&file.metadata()?, &format!("{description} handle"))?;
    require_same_metadata(&opened, &handle_after, description)?;
    let path_after = private_file_snapshot(path, owner_uid, description)?;
    require_same_metadata(&handle_after, &path_after, description)?;
    Ok(bytes)
}

fn require_stale_factory_lock(path: &Path, owner_uid: u32) -> Result<()> {
    require_stale_factory_lock_with(path, owner_uid, factory_pid_is_live)
}

fn require_stale_factory_lock_with<F>(path: &Path, owner_uid: u32, probe: F) -> Result<()>
where
    F: FnOnce(u32) -> Result<bool>,
{
    const MAX_LOCK_BYTES: u64 = 32;
    let before = private_file_snapshot(path, owner_uid, "factory lock")?;
    let bytes = read_private_file(path, owner_uid, MAX_LOCK_BYTES, "factory lock")?;
    let contents = std::str::from_utf8(&bytes)
        .map_err(|_| SheprdError::Message("factory lock PID is not valid UTF-8".into()))?;
    let pid = contents
        .trim()
        .parse::<u32>()
        .ok()
        .filter(|pid| *pid > 0)
        .ok_or_else(|| SheprdError::Message("factory lock PID is malformed".into()))?;
    if contents != format!("{pid}\n") {
        return Err(SheprdError::Message("factory lock PID is malformed".into()));
    }
    if probe(pid)? {
        return Err(SheprdError::Message(format!(
            "factory statistics are unavailable while live PID {pid} owns the factory lock"
        )));
    }
    let after = private_file_snapshot(path, owner_uid, "factory lock")?;
    require_same_metadata(&before, &after, "factory lock")?;
    Ok(())
}

fn factory_pid_is_live(pid: u32) -> Result<bool> {
    if pid == std::process::id() {
        return Ok(true);
    }
    if pid > i32::MAX as u32 {
        return Err(SheprdError::Message(format!(
            "could not verify factory lock PID {pid}: PID exceeds the supported range"
        )));
    }
    let output = Command::new("/bin/ps")
        .args(["-p", &pid.to_string(), "-o", "pid="])
        .output()
        .map_err(|error| {
            SheprdError::Message(format!("could not verify factory lock PID {pid}: {error}"))
        })?;
    if output.status.success() {
        let observed = String::from_utf8(output.stdout)
            .ok()
            .and_then(|value| value.trim().parse::<u32>().ok());
        return match observed {
            Some(observed) if observed == pid => Ok(true),
            _ => Err(SheprdError::Message(format!(
                "could not verify factory lock PID {pid}: ps returned inconsistent output"
            ))),
        };
    }
    if output.status.code() == Some(1) && output.stdout.is_empty() && output.stderr.is_empty() {
        return Ok(false);
    }
    Err(SheprdError::Message(format!(
        "could not verify factory lock PID {pid}: ps exited with {}",
        output
            .status
            .code()
            .map_or_else(|| "no status".to_string(), |code| code.to_string())
    )))
}

fn parse_observed_receipt(
    bytes: &[u8],
    project: &Project,
    run_id: &str,
    receipt_path: &Path,
    trace_path: &Path,
) -> Result<ObservedRun> {
    let value: Value = serde_json::from_slice(bytes)?;
    let version = value
        .get("schema_version")
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            SheprdError::Message("factory receipt has no valid schema_version".into())
        })?;
    match version {
        1 => {
            let receipt: LegacyFactoryReceipt = serde_json::from_value(value)?;
            validate_legacy_receipt(&receipt, project, run_id, receipt_path, trace_path)?;
            Ok(ObservedRun {
                run_id: receipt.run_id.clone(),
                task: receipt.task.clone(),
                task_reference: receipt
                    .plan
                    .as_ref()
                    .and_then(|plan| plan.task_reference.clone()),
                skill_selection_mode: receipt
                    .plan
                    .as_ref()
                    .map_or(SkillSelectionMode::None, |plan| plan.skill_selection_mode),
                selected_skills: receipt
                    .plan
                    .as_ref()
                    .map_or_else(Vec::new, |plan| plan.selected_skills.clone()),
                allow_paths: receipt.allow_paths.clone(),
                check_commands: receipt.check_commands.clone(),
                changed_paths: receipt.changed_paths.clone(),
                accepted: receipt.accepted,
                review_outcomes: ReviewOutcomes {
                    claude: review_outcome(receipt.claude_review.as_ref()),
                    opencode: review_outcome(receipt.opencode_review.as_ref()),
                },
                implementation_turn_count: receipt.implementations.len(),
                check_attempt_count: receipt.check_attempts.len(),
                failure_stage: (!receipt.accepted).then(|| "legacy_unknown".to_string()),
                elapsed_ms: None,
                cost: None,
            })
        }
        2 => {
            let receipt: FactoryReceipt = serde_json::from_value(value)?;
            validate_receipt(&receipt, project, run_id, receipt_path, trace_path)?;
            Ok(ObservedRun {
                run_id: receipt.run_id.clone(),
                task: receipt.task.clone(),
                task_reference: receipt
                    .plan
                    .as_ref()
                    .and_then(|plan| plan.task_reference.clone()),
                skill_selection_mode: receipt
                    .plan
                    .as_ref()
                    .map_or(SkillSelectionMode::None, |plan| plan.skill_selection_mode),
                selected_skills: receipt
                    .plan
                    .as_ref()
                    .map_or_else(Vec::new, |plan| plan.selected_skills.clone()),
                allow_paths: receipt.allow_paths.clone(),
                check_commands: receipt.check_commands.clone(),
                changed_paths: receipt.changed_paths.clone(),
                accepted: receipt.accepted,
                review_outcomes: receipt.review_outcomes.clone(),
                implementation_turn_count: receipt.implementation_turn_count,
                check_attempt_count: receipt.check_attempt_count,
                failure_stage: receipt.failure_stage.map(|stage| stage.label().to_string()),
                elapsed_ms: Some(receipt.elapsed_ms),
                cost: receipt.cost.authoritative,
            })
        }
        _ => Err(SheprdError::Message(format!(
            "unsupported factory receipt schema_version: {version}"
        ))),
    }
}

fn validate_legacy_receipt(
    receipt: &LegacyFactoryReceipt,
    project: &Project,
    run_id: &str,
    receipt_path: &Path,
    trace_path: &Path,
) -> Result<()> {
    if receipt.schema_version != 1 {
        return Err(SheprdError::Message(
            "legacy factory receipt schema is inconsistent".into(),
        ));
    }
    validate_embedded_plan(receipt.plan.as_ref(), &receipt.allow_paths)?;
    validate_receipt_identity(
        &receipt.run_id,
        &receipt.project,
        &receipt.receipt_path,
        &receipt.trace_path,
        project,
        run_id,
        receipt_path,
        trace_path,
    )?;
    validate_run_semantics(
        receipt.accepted,
        receipt.failure.as_deref(),
        receipt.base_unchanged,
        receipt.worker_head_unchanged,
        &receipt.implementations,
        &receipt.check_attempts,
        receipt.claude_review.as_ref(),
        receipt.opencode_review.as_ref(),
    )
}

fn validate_receipt(
    receipt: &FactoryReceipt,
    project: &Project,
    run_id: &str,
    receipt_path: &Path,
    trace_path: &Path,
) -> Result<()> {
    if receipt.schema_version != FACTORY_RECEIPT_SCHEMA_VERSION
        || receipt.implementation_turn_count != receipt.implementations.len()
        || receipt.check_attempt_count != receipt.check_attempts.len()
        || receipt.finished_at_unix_ms < receipt.started_at_unix_ms
        || receipt.accepted != (receipt.acceptance == AcceptanceOutcome::Accepted)
        || receipt.review_outcomes != review_outcomes(receipt)
        || receipt.accepted == receipt.failure_stage.is_some()
    {
        return Err(SheprdError::Message(
            "factory receipt observability fields are inconsistent".into(),
        ));
    }
    match (&receipt.cost.availability, &receipt.cost.authoritative) {
        (CostAvailability::Unavailable, None) => {}
        (CostAvailability::Authoritative, Some(cost))
            if !cost.source.trim().is_empty()
                && !cost.currency.trim().is_empty()
                && cost.minor_unit_scale <= 18 => {}
        _ => {
            return Err(SheprdError::Message(
                "factory receipt cost fields are inconsistent".into(),
            ))
        }
    }
    validate_embedded_plan(receipt.plan.as_ref(), &receipt.allow_paths)?;
    validate_receipt_identity(
        &receipt.run_id,
        &receipt.project,
        &receipt.receipt_path,
        &receipt.trace_path,
        project,
        run_id,
        receipt_path,
        trace_path,
    )?;
    validate_run_semantics(
        receipt.accepted,
        receipt.failure.as_deref(),
        receipt.base_unchanged,
        receipt.worker_head_unchanged,
        &receipt.implementations,
        &receipt.check_attempts,
        receipt.claude_review.as_ref(),
        receipt.opencode_review.as_ref(),
    )
}

fn validate_embedded_plan(plan: Option<&PlanEnvelope>, allow_paths: &[String]) -> Result<()> {
    if let Some(plan) = plan {
        let allowed = normalize_allow_paths(allow_paths)?;
        validate_plan(plan, &allowed)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_receipt_identity(
    receipt_run_id: &str,
    receipt_project: &str,
    recorded_receipt_path: &str,
    recorded_trace_path: &str,
    project: &Project,
    run_id: &str,
    receipt_path: &Path,
    trace_path: &Path,
) -> Result<()> {
    if receipt_run_id != run_id
        || receipt_project != project.name
        || Path::new(recorded_receipt_path) != receipt_path
        || Path::new(recorded_trace_path) != trace_path
    {
        return Err(SheprdError::Message(
            "factory receipt identity is inconsistent with its state path".into(),
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_run_semantics(
    accepted: bool,
    failure: Option<&str>,
    base_unchanged: bool,
    worker_head_unchanged: bool,
    implementations: &[ImplementationEnvelope],
    check_attempts: &[CheckAttempt],
    claude_review: Option<&ReviewEnvelope>,
    opencode_review: Option<&ReviewEnvelope>,
) -> Result<()> {
    if !accepted && failure.is_none() {
        return Err(SheprdError::Message(
            "rejected factory receipt has no failure".into(),
        ));
    }
    for attempt in check_attempts {
        if attempt.implementation_turn == 0
            || attempt.implementation_turn > implementations.len()
            || attempt.results.is_empty()
        {
            return Err(SheprdError::Message(
                "factory receipt check attempts are inconsistent".into(),
            ));
        }
    }
    if accepted {
        let checks_pass = check_attempts
            .last()
            .is_some_and(|attempt| attempt.results.iter().all(|result| result.success));
        let reviews_approve = claude_review.is_some_and(|review| review.approved)
            && opencode_review.is_some_and(|review| review.approved);
        if failure.is_some()
            || !base_unchanged
            || !worker_head_unchanged
            || !checks_pass
            || !reviews_approve
        {
            return Err(SheprdError::Message(
                "accepted factory receipt has inconsistent evidence".into(),
            ));
        }
    }
    Ok(())
}

fn factory_state_root(project: &Project) -> Result<PathBuf> {
    let project_root = factory_project_state_path(project)?;
    let factory_root = project_root
        .parent()
        .ok_or_else(|| SheprdError::Message("invalid factory state path".into()))?;
    create_private_dir(factory_root)?;
    create_private_dir(&project_root)?;
    Ok(project_root)
}

fn factory_project_state_path(project: &Project) -> Result<PathBuf> {
    let state_root = if let Some(path) = std::env::var_os("SHEPRD_STATE_DIR") {
        PathBuf::from(path)
    } else {
        let home = std::env::var_os("HOME").ok_or(SheprdError::MissingHome)?;
        PathBuf::from(home).join(".local/state/sheprd")
    };
    let factory_root = state_root.join("factory");
    Ok(factory_root.join(short_hash(&project.path)))
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
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{nanos}-{}", std::process::id())
}

fn unix_time_ms() -> Result<u64> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| SheprdError::Message("system clock is before the Unix epoch".into()))?
        .as_millis();
    u64::try_from(millis).map_err(|_| SheprdError::Message("system timestamp overflow".into()))
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
                schema_version: FACTORY_ENVELOPE_SCHEMA_VERSION,
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
    if pid == 0 {
        return true;
    }
    factory_pid_is_live(pid).unwrap_or(true)
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

    fn test_plan() -> PlanEnvelope {
        PlanEnvelope {
            schema_version: FACTORY_ENVELOPE_SCHEMA_VERSION,
            kind: "plan".into(),
            nonce: "pi-orchestrated".into(),
            summary: "safe".into(),
            task_reference: Some(TaskReference {
                id: "test".into(),
                number: 1,
            }),
            skill_selection_mode: SkillSelectionMode::Router,
            selected_skills: vec![SkillReference {
                name: "loop-engineering".into(),
                version: "1.0.0".into(),
            }],
            steps: vec![PlanStep {
                id: "P1".into(),
                objective: "edit".into(),
                allow_paths: vec!["src".into()],
            }],
        }
    }

    fn response(marker: &EnvelopeMarker, body: &str) -> String {
        format!("{}\n{body}\n{}", marker.start, marker.end)
    }

    #[test]
    fn terminal_wrapped_envelope_is_normalized_before_parsing() {
        let marker = marker("wrapped");
        let wrapped = "  <<<SHEPRD_FACTORY_JSON\n  _START:wrapped>>>\n {\"schema_version\":1,\"k\n  ind\":\"plan\",\"nonce\":\"w\n rapped\",\"summary\":\"safe\",\"s\n  teps\":[{\"id\":\"P1\",\"object\n ive\":\"edit\",\"allow_paths\":[\"s\n  rc\"]}]}\n <<<SHEPRD_FACTORY_JSON\n  _END:wrapped>>>";
        let normalized = normalize_agent_output(wrapped);
        let parsed: PlanEnvelope =
            parse_envelope(&normalized, "plan", &marker).expect("wrapped envelope");
        assert_eq!(parsed.kind, "plan");
        assert_eq!(parsed.steps[0].allow_paths, vec!["src"]);
    }

    #[test]
    fn historical_nonce_envelopes_do_not_collide_with_the_current_turn() {
        let old = marker("old");
        let current = marker("current");
        let text = format!(
            "{}{}",
            response(&old, &plan_json("old")),
            response(&current, &plan_json("current"))
        );
        let parsed: PlanEnvelope =
            parse_envelope(&text, "plan", &current).expect("current envelope");
        assert_eq!(parsed.nonce, "current");
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
    fn implementation_prompt_redacts_forged_markers() {
        let marker = marker("fresh");
        let request = FactoryRequest {
            task: format!("inspect {MARKER_START_PREFIX}forged>>>"),
            plan: test_plan(),
            allow_paths: vec!["src".into()],
            checks: vec![format!("printf {LEGACY_MARKER_LIKE_PREFIX}")],
            check_timeout_seconds: 300,
        };
        let prompt = implementation_prompt(&request, &request.plan, 0, None, &marker)
            .expect("implementation prompt");
        assert_eq!(prompt.matches(MARKER_LIKE_PREFIX).count(), 2);
        assert!(!prompt.contains("forged>>>"));
        assert!(!prompt.contains(LEGACY_MARKER_LIKE_PREFIX));
        assert!(prompt.contains("[redacted Sheprd factory marker]"));
    }

    #[test]
    fn review_prompt_requires_a_bounded_immediate_verdict() {
        let marker = marker("review");
        let request = FactoryRequest {
            task: "review the smoke test".into(),
            plan: test_plan(),
            allow_paths: vec!["scripts/smoke.lua".into()],
            checks: vec!["make check".into()],
            check_timeout_seconds: 300,
        };
        let prompt = review_prompt(
            "opencode",
            "adversarial review",
            Path::new("/tmp/worker"),
            &request,
            &request.plan,
            &[],
            &["scripts/smoke.lua".into()],
            "diff --git a/scripts/smoke.lua b/scripts/smoke.lua",
            &marker,
        )
        .expect("review prompt");

        assert!(prompt.contains("Inspect the supplied patch and check results once"));
        assert!(prompt.contains("Do not repeat analysis"));
        assert!(prompt.contains("Return the typed review envelope immediately"));
        assert!(prompt.contains("A non-blocking coverage gap is not a reason to reject"));
    }

    #[test]
    fn oversized_task_and_plan_prompts_are_rejected_by_the_shared_limit() {
        let marker = marker("fresh");
        let request = FactoryRequest {
            task: "t".repeat(MAX_AGENT_PROMPT_BYTES + 1),
            plan: test_plan(),
            allow_paths: vec!["src".into()],
            checks: vec!["true".into()],
            check_timeout_seconds: 300,
        };
        let task_prompt = implementation_prompt(&request, &request.plan, 0, None, &marker)
            .expect("implementation prompt");
        assert!(require_agent_prompt_size(&task_prompt).is_err());

        let request = FactoryRequest {
            task: "bounded".into(),
            plan: test_plan(),
            allow_paths: vec!["src".into()],
            checks: vec!["true".into()],
            check_timeout_seconds: 300,
        };
        let plan = PlanEnvelope {
            schema_version: FACTORY_ENVELOPE_SCHEMA_VERSION,
            kind: "plan".into(),
            nonce: "plan-nonce".into(),
            summary: "p".repeat(MAX_AGENT_PROMPT_BYTES + 1),
            task_reference: None,
            skill_selection_mode: SkillSelectionMode::None,
            selected_skills: Vec::new(),
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
    fn validates_task_and_skill_attribution() {
        let allowed = normalize_allow_paths(&["src".into()]).expect("allow paths");
        validate_plan(&test_plan(), &allowed).expect("valid attributed plan");

        let mut invalid = test_plan();
        invalid
            .selected_skills
            .push(invalid.selected_skills[0].clone());
        let duplicate = validate_plan(&invalid, &allowed).expect_err("duplicate skill");
        assert!(duplicate.to_string().contains("duplicate skill"));

        let mut invalid = test_plan();
        invalid.skill_selection_mode = SkillSelectionMode::None;
        let inconsistent = validate_plan(&invalid, &allowed).expect_err("none with skills");
        assert!(inconsistent.to_string().contains("selection mode is none"));

        let mut invalid = test_plan();
        invalid.task_reference = Some(TaskReference {
            id: String::new(),
            number: 0,
        });
        let task = validate_plan(&invalid, &allowed).expect_err("invalid task reference");
        assert!(task.to_string().contains("task reference"));
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
                plan: test_plan(),
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
        let snapshot = source_snapshot(repo.path(), &head, true).expect("snapshot");
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

    #[test]
    fn stale_factory_lock_probe_detects_a_metadata_race_deterministically() {
        let temp = assert_fs::TempDir::new().expect("temp");
        let lock_path = temp.path().join("factory.lock");
        let mut lock = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&lock_path)
            .expect("lock");
        writeln!(lock, "99999").expect("PID");
        lock.sync_all().expect("sync");
        let owner_uid = std::fs::metadata(temp.path()).expect("metadata").uid();
        let error = require_stale_factory_lock_with(&lock_path, owner_uid, |_| {
            OpenOptions::new()
                .append(true)
                .open(&lock_path)?
                .write_all(b"x")?;
            Ok(false)
        })
        .expect_err("racing lock");
        assert!(error.to_string().contains("factory lock changed"));
    }
}
#[test]
fn opencode_has_a_longer_structured_response_window() {
    assert_eq!(agent_response_timeout("opencode"), Duration::from_secs(300));
    assert_eq!(agent_response_timeout("codex"), Duration::from_secs(120));
}

use std::cmp::Ordering;

use serde::{Serialize, Deserialize};
use chrono::{DateTime, Local, Utc};

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ProblemType {
    Standard,
    Strict,
    #[serde(rename = "spj")]
    SpecialJudge,
    DynamicRanking
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
#[serde(deny_unknown_fields)]
pub enum MiscType {
    None {},
    Packed {
        packing: Vec<Vec<i32>>
    },
    SpecialJudge {
        special_judge: Vec<String>
    },
    DynamicRanking {
        dynamic_ranking_ratio: i32
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Problem {
    pub id: i32,
    pub name: String,
    #[serde(rename = "type")]
    pub problem_type: ProblemType,
    #[serde(rename = "desc")]
    #[serde(default)]
    pub description: String,
    pub misc: MiscType,
    pub cases: Vec<Case>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Case {
    pub score: f32,
    pub input_file: String,
    pub answer_file: String,
    pub time_limit: u64,
    pub memory_limit: u64
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Language {
    pub name: String,
    pub file_name: String,
    pub command: Vec<String>,
}

impl Language {
    pub fn expand_command(&self, input: &str, output: &str) -> Vec<String> {
        self.command.iter().map(|segment| { 
            match segment.as_str() {
                "%INPUT%" => input.into(),
                "%OUTPUT%" => output.into(),
                _ => segment.clone()
            }
        }).collect()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerConfig {
    pub bind_address: String,
    pub bind_port: u16,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub server: ServerConfig,
    pub problems: Vec<Problem>,
    pub languages: Vec<Language>
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
pub enum JobStatus {
    Queueing,
    Running,
    Finished,
    Canceled,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
pub enum Status {
    Waiting,
    Running,
    Accepted,
    #[serde(rename = "Compilation Error")]
    CompilationError,
    #[serde(rename = "Compilation Success")]
    CompilationSuccess,
    #[serde(rename = "Wrong Answer")]
    WrongAnswer,
    #[serde(rename = "Runtime Error")]
    RuntimeError,
    #[serde(rename = "Time Limit Exceeded")]
    TimeLimitExceeded,
    #[serde(rename = "Memory Limit Exceeded")]
    MemoryLimitExceeded,
    #[serde(rename = "System Error")]
    SystemError,
    #[serde(rename = "SPJ Error")]
    SpecialJudgeError,
    Skipped
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JobRequest {
    pub source_code: String,
    pub language: String,
    pub user_id: i32,
    pub contest_id: i32,
    pub problem_id: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JobCase {
    pub id: i32,
    pub result: Status,
    pub time: u64,
    pub memory: u64,
    pub info: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Job {
    pub id: i32,
    pub created_time: DateTime<Utc>,
    pub updated_time: DateTime<Utc>,
    pub submission: JobRequest,
    pub state: JobStatus,
    pub result: Status,
    pub score: f32,
    pub cases: Vec<JobCase>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct User {
    pub id: i32,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Contest {
    pub id: i32,
    pub name: String,
    #[serde(serialize_with = "crate::serde_helper::serialize_datetime")]
    #[serde(deserialize_with ="crate::serde_helper::deserialize_datetime")]
    pub from: DateTime<Utc>,
    #[serde(serialize_with = "crate::serde_helper::serialize_datetime")]
    #[serde(deserialize_with ="crate::serde_helper::deserialize_datetime")]
    pub to: DateTime<Utc>,
    pub problem_ids: Vec<i32>,
    pub user_ids: Vec<i32>,
    pub submission_limit: i32
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ScoringRule {
    Latest,
    Highest
}

impl ScoringRule {
    pub fn choose<'a>(&self, jobs: &'a[Job]) -> Option<&'a Job> {
        match *self {
            Self::Latest => {
                jobs.iter().max_by_key(|job| job.created_time)
            }
            Self::Highest => {
                jobs.iter().max_by(|a, b| {
                    a.score.total_cmp(&b.score)
                    .then(a.created_time.cmp(&b.created_time).reverse())
                })
            }
        }
    } 
}

impl Default for ScoringRule {
    fn default() -> Self {
        Self::Latest
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case")]
pub enum TieBreaker {
    SubmissionTime,
    SubmissionCount,
    UserId
}

impl TieBreaker {
    pub(crate) fn compare(&self, a: &RankHelper, b: &RankHelper) -> Ordering {
        match *self {
            Self::SubmissionTime => {
                let a_time = a.jobs.iter().map(|job| {
                    match job { 
                        Some(job) => job.created_time.clone(),
                        None => DateTime::<Utc>::MAX_UTC
                    }
                }).min().unwrap();
                
                let b_time = b.jobs.iter().map(|job| {
                    match job { 
                        Some(job) => job.created_time.clone(),
                        None => DateTime::<Utc>::MAX_UTC
                    }
                }).min().unwrap();
                
                Ord::cmp(&a_time, &b_time)
            },
            Self::SubmissionCount => {
                let mut a_sum = 0;
                let mut b_sum = 0;
                for count in &a.job_counts { a_sum += *count; }
                for count in &b.job_counts { b_sum += *count; }
                Ord::cmp(&a_sum, &b_sum)
            },
            Self::UserId => {
                Ord::cmp(&a.user.id, &b.user.id)
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Ranking {
    pub user: User,
    pub rank: i32,
    pub scores: Vec<f32>,
}

#[derive(Debug)]
pub(crate) struct RankHelper {
    pub user: User,
    pub jobs: Vec<Option<Job>>,
    pub job_counts: Vec<i32>
}

impl RankHelper {
    pub fn score(&self) -> f32 {
        let mut score = 0f32;
        for job in &self.jobs {
            if let Some(job) = job {
                score += job.score;
            }
        }
        score
    }
}

#[derive(Serialize, Deserialize)]
pub struct TokenPayload {
    pub address: String,
    pub expires: DateTime<Local>,
    pub subject: User
}

#[derive(Debug, Clone)]
pub struct Error {
    pub code: i32,
    pub reason: &'static str,
    pub message: String,
    pub http_status: u16
}

impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        ERR_INTERNAL.clone()
    }
}

impl From<redb::CommitError> for Error {
    fn from(_: redb::CommitError) -> Self {
        ERR_EXTERNAL.clone().with_message("Redb commit error.".into())
    }
}

impl From<redb::DatabaseError> for Error {
    fn from(value: redb::DatabaseError) -> Self {
        ERR_EXTERNAL.clone().with_message(format!("Redb database error. {}", value))
    }
}

impl From<redb::TransactionError> for Error {
    fn from(_: redb::TransactionError) -> Self {
        ERR_EXTERNAL.clone().with_message("Redb transaction error.".into())
    }
}

impl From<redb::TableError> for Error {
    fn from(_: redb::TableError) -> Self {
        ERR_EXTERNAL.clone().with_message("Redb table error.".into())
    }
}

impl From<redb::StorageError> for Error {
    fn from(_: redb::StorageError) -> Self {
        ERR_EXTERNAL.clone().with_message("Redb storage error.".into())
    }
}

impl From<serde_json::Error> for Error {
    fn from(_: serde_json::Error) -> Self {
        ERR_INTERNAL.clone().with_message("JSON (de)serialization error.".into())
    }
}

impl Error {
    pub fn with_message(mut self, msg: String) -> Self {
        self.message = msg;
        self
    }
}

lazy_static::lazy_static! {

    pub static ref ERR_INVALID_ARGUMENT: Error = Error {
        code: 1,
        reason: "ERR_INVALID_ARGUMENT",
        message: "".into(),
        http_status: 400,
    };

    pub static ref ERR_INVALID_STATE: Error = Error {
        code: 2,
        reason: "ERR_INVALID_STATE",
        message: "".into(),
        http_status: 400,
    };

    pub static ref ERR_NOT_FOUND: Error = Error {
        code: 3,
        reason: "ERR_NOT_FOUND",
        message: "".into(),
        http_status: 404,
    };

    pub static ref ERR_RATE_LIMIT: Error = Error {
        code: 4,
        reason: "ERR_RATE_LIMIT",
        message: "".into(),
        http_status: 400,
    };

    pub static ref ERR_EXTERNAL: Error = Error {
        code: 5,
        reason: "ERR_EXTERNAL",
        message: "".into(),
        http_status: 500,
    };

    pub static ref ERR_INTERNAL: Error = Error {
        code: 6,
        reason: "ERR_INTERNAL",
        message: "".into(),
        http_status: 500,
    };

}

pub type Result<T> = std::result::Result<T, Error>;
use std::cmp::Ordering;
use std::collections::VecDeque;
use std::env::temp_dir;
use std::fs::{copy, create_dir_all, read_to_string, remove_dir_all, write};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread::{spawn, JoinHandle};

use chrono::Utc;
use rand::Rng;

use crate::database::Database;
use crate::judge::{standard_judge, strict_judge};
use crate::models::*;

struct Tempdir {
    path: PathBuf,
}

impl Tempdir {
    fn random_name() -> String {
        const CANDIDATES: &str = "abcdefghijklmnopqrstuvwxyz1234567890";
        let mut rng = rand::thread_rng();
        let mut file_name = String::new();
        for _ in 0..8 {
            file_name.push(CANDIDATES.chars().nth(rng.gen_range(0..CANDIDATES.len())).unwrap());
        }
        file_name
    }
    pub fn new() -> Result<Self> {
        let path = temp_dir().join(Self::random_name());
        create_dir_all(&path)?;
        Ok(Self {
            path
        })
    }
    pub fn wrap(&self, name: &str) -> String {
        self.path.join(name).to_str().unwrap().into()
    }
    pub fn random(&self) -> String {
        self.wrap(&Self::random_name())
    }
    pub fn clean(self) -> Result<()> {
        Ok(remove_dir_all(&self.path)?)
    }
}

impl Drop for Tempdir {
    fn drop(&mut self) {
        if self.path.exists() {
            let _ = remove_dir_all(&self.path);
        }
    }
}

impl Job {
    pub fn new(id: i32, submission: &JobRequest, case_len: usize) -> Self {
        let mut cases = vec![];
        for i in 0..=case_len {
            cases.push(JobCase {
                id: i as _,
                result: Status::Waiting,
                time: 0,
                memory: 0,
                info: "".into()
            })
        }
        Self {
            id,
            created_time: Utc::now(),
            updated_time: Utc::now(),
            submission: submission.clone(),
            state: JobStatus::Queueing,
            result: Status::Waiting,
            score: 0.0,
            cases
        }
    }
}

// ephemeral lock stage
fn lock<'a, T, F, R>(value: &'a Mutex<T>, op: F) -> R where F: FnOnce(&mut T) -> R + 'a {
    let mut locked = value.lock().unwrap();
    op(&mut *locked)
}

fn update_job<'a, F>(value: &'a Mutex<Job>, op: F) where F: FnOnce(&mut Job) + 'a {
    lock(value, op);
    lock(value, |job| job.updated_time = Utc::now());
}

pub struct Worker {
    pub config: Config,
    queue: Mutex<VecDeque<Arc<Mutex<Job>>>>,
    running: Mutex<bool>,
    db: Database,
    job_id: Mutex<i32>,
    user_id: Mutex<i32>,
    contest_id: Mutex<i32>,
}

impl Worker {
    pub fn new(config: Config, flush_data: bool) -> Result<Self> {
        let db = Database::new(flush_data)?;
        Ok(Self {
            config,
            queue: Mutex::new(VecDeque::new()),
            running: Mutex::new(false),
            job_id: Mutex::new(db.largest_job_id()?),
            user_id: Mutex::new(db.largest_user_id()?),
            contest_id: Mutex::new(db.largest_contest_id()?),
            db,
        })
    }
    pub fn create_job(&self, request: &JobRequest) -> Result<Job> {
        if !self.db.user_exists(request.user_id)? {
            return Err(ERR_NOT_FOUND.clone().with_message(format!("User {} not found.", request.user_id))); 
        }
        let problem = self.config.problems.iter().find(
            |problem| problem.id == request.problem_id
        );
        if problem.is_none() { 
            return Err(ERR_NOT_FOUND.clone().with_message(format!("Problem {} not found.", request.problem_id))); 
        }
        if request.contest_id != 0 {
            let contest = self.db.find_contest_by_id(request.contest_id)?;
            if contest.is_none() {
                return Err(ERR_NOT_FOUND.clone().with_message(format!("Contest {} not found.", request.contest_id))); 
            }
            let contest = contest.unwrap();
            if !contest.problem_ids.contains(&request.problem_id) || !contest.user_ids.contains(&request.user_id) {
                return Err(ERR_INVALID_ARGUMENT.clone());
            }
            let job_count = self.find_jobs(|job| {
                Ok(
                    job.submission.contest_id == request.contest_id &&
                    job.submission.problem_id == request.problem_id &&
                    job.submission.user_id == request.user_id
                )
            })?.len() as i32;
            if job_count >= contest.submission_limit {
                return Err(ERR_RATE_LIMIT.clone());
            }
        }
        let language = self.config.languages.iter().find(
            |lang| lang.name == request.language
        );
        if language.is_none() {
            return Err(ERR_NOT_FOUND.clone().with_message(format!("Language {} not found.", request.language)));
        }
        Ok(Job::new({ 
            let mut v = self.job_id.lock().unwrap();
            *v += 1;
            *v
        }, request, problem.unwrap().cases.len()))
    }
    pub fn run(&self, job: &Mutex<Job>) {
        let result = self.run_unsafe(job);
        if result.is_err() {
            update_job(job, |job| {
                job.result = Status::SystemError;
            });
        }
    }
    fn run_unsafe(&self, job: &Mutex<Job>) -> Result<()> {
        update_job(job, |job| { 
            job.result = Status::Running;
            job.state = JobStatus::Running;
        });
        let language = self.config.languages.iter().find(
            |lang| lock(job, |job| lang.name == job.submission.language)
        );
        let language = language.unwrap();
        let problem = self.config.problems.iter().find(
            |problem| problem.id == lock(job, |job| job.submission.problem_id)
        );
        let problem = problem.unwrap();

        let tempdir = Tempdir::new()?;
        let source_file_name = tempdir.wrap(&language.file_name);
        let exe_file_name = tempdir.random();
        let input_file_name = tempdir.random();
        let output_file_name = tempdir.random();
        write(&source_file_name, lock(job, |job| job.submission.source_code.clone()))?;

        if !language.compile(&source_file_name, &exe_file_name) {
            update_job(job, |job| {
                job.cases[0].result = Status::CompilationError;
                job.result = Status::CompilationError;
                job.state = JobStatus::Finished;
            });
            tempdir.clean()?;
            return Ok(())
        }

        update_job(job, |job| {
            job.cases[0].result = Status::CompilationSuccess;
        });

        if let MiscType::Packed { packing } = &problem.misc {
            for group in packing {
                let mut group_valid = true;
                let mut group_score = 0f32;
                for i in group {
                    let i = *i as usize;
                    let case = &problem.cases[i-1];
                    if !group_valid {
                        update_job(job, |job| {
                            job.cases[i].result = Status::Skipped;
                        });
                        continue;
                    }
                    update_job(job, |job| {
                        job.cases[i].result = Status::Running;
                    });
                    copy(&case.input_file, &input_file_name)?;
                    let (result, resources) = 
                        case.run(&exe_file_name, &input_file_name, &output_file_name);
                    match result {
                        Ok(_) => {
                            match match problem.problem_type {
                                ProblemType::Standard => {
                                    let got = read_to_string(&output_file_name)?;
                                    let expected = read_to_string(&case.answer_file)?;
                                    standard_judge(&got, &expected)
                                }
                                ProblemType::Strict => {
                                    let got = read_to_string(&output_file_name)?;
                                    let expected = read_to_string(&case.answer_file)?;
                                    strict_judge(&got, &expected)
                                }
                                ProblemType::SpecialJudge => {
                                    if let MiscType::SpecialJudge { special_judge } = &problem.misc {
                                        let special_judge = special_judge.iter().map(|segment| {
                                            dbg!(segment);
                                            match segment.as_str() {
                                                "%OUTPUT%" => &output_file_name,
                                                "%ANSWER%" => &case.answer_file,
                                                s => s
                                            }
                                        }).collect::<Vec<&str>>();
                                        let output =
                                            Command::new(&special_judge[0])
                                            .args(&special_judge[1..])
                                            .output()?;
                                        let stdout = String::from_utf8(output.stdout).unwrap();
                                        let lines = stdout.split("\n").collect::<Vec<&str>>();
                                        if lines.len() != 2 {
                                            Status::SpecialJudgeError
                                        }
                                        else {
                                            let status = serde_json::from_str(&format!("\"{}\"", lines[0]));
                                            match status {
                                                Ok(status) => {
                                                    update_job(job, |job| {
                                                        job.cases[i].info = lines[1].into();
                                                    });
                                                    status
                                                }
                                                Err(_) => Status::SpecialJudgeError
                                            }
                                        }
                                    }
                                    else {
                                        Status::SystemError
                                    }
                                }
                                _ => Status::Accepted
                            } {
                                Status::Accepted => {
                                    update_job(job, |job| {
                                        job.cases[i].result = Status::Accepted;
                                        group_score += case.score;
                                    });
                                }
                                status => {
                                    group_valid = false;
                                    group_score = 0f32;
                                    update_job(job, |job| {
                                        job.cases[i].result = status;
                                        if job.result == Status::Running {
                                            job.result = status;
                                        }
                                    });
                                }
                            }
                        }
                        Err(status) => {
                            group_valid = false;
                            group_score = 0f32;
                            update_job(job, |job| {
                                job.cases[i].result = status;
                                if job.result == Status::Running {
                                    job.result = status;
                                }
                            });
                        }
                    }
                    update_job(job, |job| {
                        job.cases[i].time = resources.time;
                        job.cases[i].memory = resources.memory;
                    });
                }
                update_job(job, |job| {
                    job.score += group_score;
                });
            }
        }
        else {
            for (i, case) in problem.cases.iter().enumerate() {
                update_job(job, |job| {
                    job.cases[i+1].result = Status::Running;
                });
                copy(&case.input_file, &input_file_name)?;
                let (result, resources) = 
                    case.run(&exe_file_name, &input_file_name, &output_file_name);
                match result {
                    Ok(_) => {
                        match match problem.problem_type {
                            ProblemType::Standard => {
                                let got = read_to_string(&output_file_name)?;
                                let expected = read_to_string(&case.answer_file)?;
                                standard_judge(&got, &expected)
                            }
                            ProblemType::Strict => {
                                let got = read_to_string(&output_file_name)?;
                                let expected = read_to_string(&case.answer_file)?;
                                strict_judge(&got, &expected)
                            }
                            ProblemType::SpecialJudge => {
                                if let MiscType::SpecialJudge { special_judge } = &problem.misc {
                                    let special_judge = special_judge.iter().map(|segment| {
                                        dbg!(segment);
                                        match segment.as_str() {
                                            "%OUTPUT%" => &output_file_name,
                                            "%ANSWER%" => &case.answer_file,
                                            s => s
                                        }
                                    }).collect::<Vec<&str>>();
                                    dbg!(&special_judge);
                                    let output =
                                        Command::new(&special_judge[0])
                                        .args(&special_judge[1..])
                                        .output()?;
                                    dbg!(&output);
                                    let stdout = String::from_utf8(output.stdout).unwrap();
                                    let lines = stdout.split_terminator("\n").collect::<Vec<&str>>();
                                    if lines.len() != 2 {
                                        Status::SpecialJudgeError
                                    }
                                    else {
                                        let status = serde_json::from_str(&format!("\"{}\"", lines[0]));
                                        match status {
                                            Ok(status) => {
                                                update_job(job, |job| {
                                                    job.cases[i+1].info = lines[1].into();
                                                });
                                                status
                                            }
                                            Err(_) => Status::SpecialJudgeError
                                        }
                                    }
                                }
                                else {
                                    Status::SystemError
                                }
                            }
                            _ => Status::Accepted
                        } {
                            Status::Accepted => {
                                update_job(job, |job| {
                                    job.cases[i+1].result = Status::Accepted;
                                    job.score += case.score;
                                });
                            }
                            status => {
                                update_job(job, |job| {
                                    job.cases[i+1].result = status;
                                    if job.result == Status::Running {
                                        job.result = status;
                                    }
                                });
                            }
                        }
                    }
                    Err(status) => {
                        update_job(job, |job| {
                            job.cases[i+1].result = status;
                            if job.result == Status::Running {
                                job.result = status;
                            }
                        });
                    }
                }
                update_job(job, |job| {
                    job.cases[i+1].time = resources.time;
                    job.cases[i+1].memory = resources.memory;
                });
            }
        }
        update_job(job, |job| {
            if job.result == Status::Running {
                job.result = Status::Accepted;
            }
            job.state = JobStatus::Finished;
        });

        Ok(())
    }
    pub fn start(self: &Arc<Self>) -> JoinHandle<()> {
        let this = Arc::clone(self);
        lock(&self.running, |v| *v = true);
        spawn(move || {
            while lock(&this.running, |v| *v) {
                if lock(&this.queue, |queue| queue.is_empty()) {
                    continue;
                }
                let job = lock(&this.queue, |queue| Arc::clone(&queue.front().unwrap()));
                this.run(&job);
                let job = lock(&this.queue, |queue| queue.pop_front()).unwrap();
                lock(&job, |job| this.db.put_job(job)).ok(); // discard this error, as there is no way to handle it.
            }
        })
    }
    pub fn push_job(&self, job: Arc<Mutex<Job>>) {
        lock(&self.queue, |queue| queue.push_back(job));
    }
    pub fn find_job<F>(&self, mut predicate: F) -> Result<Option<Job>> where F: FnMut(&Job) -> Result<bool> { // snapshot
        if let Some(job) = lock(&self.queue, |queue| -> Result<Option<Job>> {
            for job in queue {
                if let Some(job) = lock(job, |job| -> Result<Option<Job>> {
                    if predicate(job)? {
                        return Ok(Some(job.clone()));
                    }
                    Ok(None)
                })? {
                    return Ok(Some(job));
                }
            }
            Ok(None)
        })? { 
            return Ok(Some(job));
        }
        self.db.find_job(predicate)
    }
    pub fn find_jobs<F>(&self, mut predicate: F) -> Result<Vec<Job>> where F: FnMut(&Job) -> Result<bool> {
        let mut result = vec![];
        lock(&self.queue, |queue| -> Result<()> {
            for job in queue {
                lock(job, |job| -> Result<()> {
                    if predicate(job)? {
                        result.push(job.clone());
                    }
                    Ok(())
                })?;
            }
            Ok(())
        })?;
        result.extend(self.db.find_jobs(predicate)?);
        Ok(result)
    }
    pub fn stop(&self) {
        lock(&self.running, |v| *v = false);
    }
    pub fn cancel_job(&self, id: i32) -> Result<()> {
        match self.find_job(move |job| Ok(job.id == id))? {
            Some(job) => {
                if job.state != JobStatus::Queueing {
                    return Err(ERR_INVALID_STATE.clone().with_message(format!("Job {} not queueing.", id)));
                }
                lock(&self.queue, |queue| {
                    let mut index = None;
                    for (i, job) in queue.iter().enumerate() {
                        if lock(job, |job| job.id == id) {
                            index = Some(i);
                            break;
                        }
                    }
                    if let Some(i) = index { queue.remove(i); }
                });
                Ok(())
            },
            None => {
                Err(ERR_NOT_FOUND.clone().with_message(format!("Job {} not found.", id)))
            }
        }
    }
    pub fn rerun_job(&self, id: i32) -> Result<Job> {
        match self.db.find_job(move |job| Ok(job.id == id))? {
            Some(mut job) => {
                if job.state != JobStatus::Finished {
                    return Err(ERR_INVALID_STATE.clone().with_message(format!("Job {} not finished.", id)));
                }
                job.state = JobStatus::Queueing;
                job.result = Status::Waiting;
                job.score = 0.0;
                job.updated_time = Utc::now();
                for case in &mut job.cases {
                    case.info = "".into();
                    case.memory = 0;
                    case.time = 0;
                    case.result = Status::Waiting;
                }
                self.push_job(Arc::new(Mutex::new(job.clone())));
                Ok(job)
            },
            None => {
                Err(ERR_NOT_FOUND.clone().with_message(format!("Job {} not found.", id)))
            }
        }
    }
    pub fn create_user(&self, name: &str) -> Result<User> {
        if matches!(self.db.find_user_by_name(name)?, Some(_)) {
            return Err(ERR_INVALID_ARGUMENT.clone().with_message(format!("User name '{}' already exists.", name)));
        }
        let user = User {
            id: { 
                let mut v = self.user_id.lock().unwrap();
                *v += 1;
                *v
            },
            name: name.into(),
        };
        self.db.put_user(&user)?;
        Ok(user)
    }
    pub fn update_user(&self, user: &User) -> Result<()> {
        if let Some(old_user) = self.db.find_user_by_name(&user.name)? {
            if old_user.id != user.id {
                return Err(ERR_INVALID_ARGUMENT.clone().with_message(format!("User name '{}' already exists.", &user.name)));
            }
        }
        if !self.db.user_exists(user.id)? {
            return Err(ERR_NOT_FOUND.clone().with_message(format!("User {} not found.", user.id)))
        }
        self.db.put_user(user)?;
        Ok(())
    }
    pub fn next_contest_id(&self) -> i32 {
        let mut v = self.contest_id.lock().unwrap();
        *v += 1;
        *v
    }
    pub fn global_ranklist(&self, scoring_role: ScoringRule, tie_breaker: Option<TieBreaker>) -> Result<Vec<Ranking>> {
        let users = self.db.list_users()?;
        let mut helpers = vec![];
        let mut result: Vec<Ranking> = vec![];
        for user in &users {
            let mut helper = RankHelper {
                user: user.clone(),
                jobs: vec![],
                job_counts: vec![]
            };
            for problem in &self.config.problems {
                let jobs = self.db.find_jobs(
                    |job| Ok(job.submission.user_id == user.id && job.submission.problem_id == problem.id)
                )?;
                let job = scoring_role.choose(&jobs);
                helper.jobs.push(job.map(|job| job.clone()));
                helper.job_counts.push(jobs.len() as _);
            }
            helpers.push(helper);
        }
        helpers.sort_by(|a, b| f32::total_cmp(&a.score(), &b.score()).reverse().then(match tie_breaker {
            Some(ref tie_breaker) => tie_breaker.compare(a, b),
            None => Ordering::Equal
        }));
        for (i, helper) in helpers.iter().enumerate() {
            if i != 0 && helper.score() == helpers[i-1].score() &&
            match tie_breaker {
                Some(ref tie_breaker) => tie_breaker.compare(helper, &helpers[i-1]).is_eq(),
                None => true
            } {
                result.push(Ranking {
                    user: helper.user.clone(),
                    rank: result.last().unwrap().rank,
                    scores: helper.jobs.iter().map(|job| match job {
                        Some(job) => job.score,
                        None => 0f32
                    }).collect()
                })
            }
            else {
                result.push(Ranking {
                    user: helper.user.clone(),
                    rank: (i+1) as i32,
                    scores: helper.jobs.iter().map(|job| match job {
                        Some(job) => job.score,
                        None => 0f32
                    }).collect()
                })
            }
        }
        Ok(result)
    }
    pub fn contest_ranklist(&self, contest_id: i32, scoring_role: ScoringRule, tie_breaker: Option<TieBreaker>) -> Result<Vec<Ranking>> {
        let contest = self.db.find_contest_by_id(contest_id)?;
        if contest.is_none() {
            return Err(ERR_NOT_FOUND.clone().with_message(format!("Contest {} not found.", contest_id)));
        }
        let contest = contest.unwrap();
        let users = self.db.find_users(|user| contest.user_ids.contains(&user.id))?;
        let mut helpers = vec![];
        let mut result: Vec<Ranking> = vec![];
        for user in &users {
            let mut helper = RankHelper {
                user: user.clone(),
                jobs: vec![],
                job_counts: vec![]
            };
            for problem_id in &contest.problem_ids 
            {
                let problem = self.config.problems.iter().find(|problem| problem.id == *problem_id).unwrap();
                let jobs = self.db.find_jobs(
                    |job| Ok(job.submission.user_id == user.id && job.submission.problem_id == problem.id)
                )?;
                let job = scoring_role.choose(&jobs);
                helper.jobs.push(job.map(|job| job.clone()));
                helper.job_counts.push(jobs.len() as _);
            }
            helpers.push(helper);
        }
        helpers.sort_by(|a, b| f32::total_cmp(&a.score(), &b.score()).reverse().then(match tie_breaker {
            Some(ref tie_breaker) => tie_breaker.compare(a, b),
            None => Ordering::Equal
        }));
        for (i, helper) in helpers.iter().enumerate() {
            if i != 0 && helper.score() == helpers[i-1].score() &&
            match tie_breaker {
                Some(ref tie_breaker) => tie_breaker.compare(helper, &helpers[i-1]).is_eq(),
                None => true
            } {
                result.push(Ranking {
                    user: helper.user.clone(),
                    rank: result.last().unwrap().rank,
                    scores: helper.jobs.iter().map(|job| match job {
                        Some(job) => job.score,
                        None => 0f32
                    }).collect()
                })
            }
            else {
                result.push(Ranking {
                    user: helper.user.clone(),
                    rank: (i+1) as i32,
                    scores: helper.jobs.iter().map(|job| match job {
                        Some(job) => job.score,
                        None => 0f32
                    }).collect()
                })
            }
        }
        Ok(result)
    }
    pub fn find_problems<F>(&self, mut predicate: F) -> Vec<&Problem> where F: FnMut(&Problem) -> bool {
        self.config.problems.iter().filter(|problem| predicate(*problem)).collect()
    }
    pub fn get_contest_problems(&self, contest_id: i32) -> Result<Vec<&Problem>> {
        let contest = self.db.find_contest_by_id(contest_id)?;
        match contest {
            Some(contest) => Ok(self.find_problems(|problem| contest.problem_ids.contains(&problem.id))),
            None => Err(ERR_NOT_FOUND.clone().with_message(format!("Contest {} not found.", contest_id)))
        }
    }
    pub fn database(&self) -> &Database {
        &self.db
    }
}
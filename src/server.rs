use actix_web::*;
use chrono::{DateTime, Days, Local, Utc};
use cookie::time::Duration;
use cookie::Cookie;
use http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::{Arc, Mutex};
use crate::models::*;
use crate::tokens::{Token, TokenFactory};
use crate::worker::Worker;

#[get("/greet")]
pub async fn greet() -> impl Responder {
    "Hello, world!"
}

#[post("/internal/exit")]
#[allow(unreachable_code)]
pub async fn exit(req: HttpRequest) -> impl Responder {
    let worker = req.app_data::<Arc<Worker>>().unwrap();
    worker.stop();
    std::process::exit(0);
    "Exited"
}

#[post("/jobs")]
pub async fn post_jobs(req: HttpRequest, request: web::Json<JobRequest>) -> impl Responder {
    let enable_auth = *req.app_data::<bool>().unwrap();
    let factory = req.app_data::<TokenFactory>().unwrap();
    if enable_auth {
        if let Some(cookie) = req.cookie("rustoj-token") {
            let token = match Token::parse(cookie.value()) {
                Ok(token) => token,
                Err(e) => {
                    return HttpResponse::build(StatusCode::from_u16(e.http_status).unwrap()).json(json!({
                        "code": e.code,
                        "reason": e.reason,
                        "message": e.message
                    }));
                }
            };
            let payload = match factory.parse(&token) {
                Ok(payload) => payload,
                Err(e) => {
                    return HttpResponse::build(StatusCode::from_u16(e.http_status).unwrap()).json(json!({
                        "code": e.code,
                        "reason": e.reason,
                        "message": e.message
                    }));
                }
            };
            if payload.address != req.peer_addr().unwrap().ip().to_string() || payload.subject.id != request.user_id {
                return HttpResponse::build(StatusCode::from_u16(ERR_INVALID_ARGUMENT.http_status).unwrap()).json(json!({
                    "code": ERR_INVALID_ARGUMENT.code,
                    "reason": ERR_INVALID_ARGUMENT.reason,
                    "message": "Not your token."
                }));
            }
        }
        else {
            return HttpResponse::build(StatusCode::from_u16(ERR_INVALID_ARGUMENT.http_status).unwrap()).json(json!({
                "code": ERR_INVALID_ARGUMENT.code,
                "reason": ERR_INVALID_ARGUMENT.reason,
                "message": "Token required."
            }));
        }
    }
    let worker = req.app_data::<Arc<Worker>>().unwrap();
    let job = worker.create_job(&request);
    if let Err(e) = job {
        return HttpResponse::build(StatusCode::from_u16(e.http_status).unwrap()).json(json!({
            "code": e.code,
            "reason": e.reason,
            "message": e.message
        }));
    }
    let job = job.unwrap();
    let response = HttpResponse::Ok().json(&job);
    let job = Arc::new(Mutex::new(job));
    worker.push_job(Arc::clone(&job));
    response
}

#[derive(Serialize, Deserialize)]
struct JobQuery {
    user_id: Option<i32>,
    user_name: Option<String>,
    contest_id: Option<i32>,
    problem_id: Option<i32>,
    language: Option<String>,
    from: Option<chrono::DateTime<Utc>>,
    to: Option<chrono::DateTime<Utc>>,
    state: Option<JobStatus>,
    result: Option<Status>
}


#[get("/jobs")]
pub async fn get_jobs(req: HttpRequest, query: web::Query<JobQuery>) -> impl Responder {
    let worker = req.app_data::<Arc<Worker>>().unwrap();
    match worker.find_jobs(move |job| {
        if let Some(user_id) = query.user_id {
            if job.submission.user_id != user_id { return Ok(false); }
        }
        if let Some(ref user_name) = query.user_name {
            match worker.database().find_user_by_name(user_name)? {
                Some(user) => {
                    if job.submission.user_id != user.id { return Ok(false); }
                }
                None => { return Ok(false); }
            }
        }
        if let Some(contest_id) = query.contest_id {
            if job.submission.contest_id != contest_id { return Ok(false); }
        }
        if let Some(problem_id) = query.problem_id {
            if job.submission.problem_id != problem_id { return Ok(false); }
        }
        if let Some(ref language) = query.language { 
            if job.submission.language != *language { return Ok(false); }
        }
        if let Some(from) = query.from {
            if job.created_time < from { return Ok(false); }
        }
        if let Some(to) = query.to {
            if job.created_time > to { return Ok(false); }
        }
        if let Some(state) = query.state {
            if job.state != state { return Ok(false); }
        }
        if let Some(result) = query.result {
            if job.result != result { return Ok(false); }
        }
        Ok(true)
    }) {
        Ok(jobs) => {
            HttpResponse::Ok().json(jobs)
        }
        Err(e) => {
            HttpResponse::build(StatusCode::from_u16(e.http_status).unwrap()).json(json!({
                "code": e.code,
                "reason": e.reason,
                "message": e.message
            }))
        }
    }
}

#[get("/jobs/{id}")]
pub async fn get_job_by_id(req: HttpRequest, path: web::Path<i32>) -> impl Responder {
    let id = path.into_inner();
    let worker = req.app_data::<Arc<Worker>>().unwrap();
    match worker.find_job(move |job| Ok(job.id == id)) {
        Ok(job) => {
            if let Some(job) = job {
                HttpResponse::Ok().json(job)
            }
            else {
                HttpResponse::NotFound().json(json!({
                    "code": ERR_NOT_FOUND.code,
                    "reason": ERR_NOT_FOUND.reason,
                    "message": format!("Job {} not found.", id)
                }))
            }
        }
        Err(e) => {
            HttpResponse::build(StatusCode::from_u16(e.http_status).unwrap()).json(json!({
                "code": e.code,
                "reason": e.reason,
                "message": e.message
            }))
        }
    }
}

#[put("/jobs/{id}")]
pub async fn rerun_job(req: HttpRequest, path: web::Path<i32>) -> impl Responder {
    let id = path.into_inner();
    let worker = req.app_data::<Arc<Worker>>().unwrap();
    match worker.rerun_job(id) {
        Ok(job) => {
            HttpResponse::Ok().json(job)
        }
        Err(e) => {
            HttpResponse::build(StatusCode::from_u16(e.http_status).unwrap()).json(json!({
                "code": e.code,
                "reason": e.reason,
                "message": e.message
            }))
        }
    }
}

#[delete("/jobs/{id}")]
pub async fn cancel_job(req: HttpRequest, path: web::Path<i32>) -> impl Responder {
    let id = path.into_inner();
    let worker = req.app_data::<Arc<Worker>>().unwrap();
    match worker.cancel_job(id) {
        Ok(_) => {
            HttpResponse::new(StatusCode::OK)
        }
        Err(e) => {
            HttpResponse::build(StatusCode::from_u16(e.http_status).unwrap()).json(json!({
                "code": e.code,
                "reason": e.reason,
                "message": e.message
            }))
        }
    }
}

#[get("/users")]
pub async fn get_users(req: HttpRequest) -> impl Responder {
    let worker = req.app_data::<Arc<Worker>>().unwrap();
    match worker.database().list_users() {
        Ok(users) => {
            HttpResponse::Ok().json(users)
        }
        Err(e) => {
            HttpResponse::build(StatusCode::from_u16(e.http_status).unwrap()).json(json!({
                "code": e.code,
                "reason": e.reason,
                "message": e.message
            }))
        }
    }
}

#[derive(Serialize, Deserialize)]
struct UserUpdate {
    id: Option<i32>,
    name: String
}

#[post("/users")]
pub async fn post_users(req: HttpRequest, update: web::Json<UserUpdate>) -> impl Responder {
    let worker = req.app_data::<Arc<Worker>>().unwrap();
    match match update.id {
        Some(id) => {
            let user = User {
                id,
                name: update.name.clone()
            };
            worker.update_user(&user).map(move |_| user)
        },
        None => {
            worker.create_user(&update.name)
        }
    } {
        Ok(user) => {
            HttpResponse::Ok().json(user)
        }
        Err(e) => {
            HttpResponse::build(StatusCode::from_u16(e.http_status).unwrap()).json(json!({
                "code": e.code,
                "reason": e.reason,
                "message": e.message
            }))
        }
    }
}

#[derive(Serialize, Deserialize)]
struct UserLogin {
    name: String
}

#[post("/users/login")]
pub async fn login(req: HttpRequest, login: web::Json<UserLogin>) -> impl Responder {
    let enable_auth = *req.app_data::<bool>().unwrap();
    if !enable_auth {
        return HttpResponse::build(StatusCode::from_u16(ERR_INTERNAL.http_status).unwrap())
            .json(json!({
                "code": ERR_INTERNAL.code,
                "reason": ERR_INTERNAL.reason,
                "message": "Authentication is disabled."
            }))
    }
    let worker = req.app_data::<Arc<Worker>>().unwrap();
    let factory = req.app_data::<TokenFactory>().unwrap();
    match worker.database().find_user_by_name(&login.name) {
        Ok(user) => {
            match user {
                Some(user) => {
                    match factory.create(&TokenPayload {
                        address: req.peer_addr().unwrap().ip().to_string(),
                        expires: Local::now().checked_add_days(Days::new(10)).unwrap(),
                        subject: user.clone()
                    }) {
                        Ok(token) => {
                            let token_string = token.to_string();
                            HttpResponse::Ok()
                                .cookie(
                                    Cookie::build("rustoj-token", &token_string)
                                    .max_age(Duration::days(10))
                                    .http_only(false)
                                    .path("/")
                                    .finish()
                                )
                                .json(json!({
                                    "token": token_string
                                }))
                        }
                        Err(e) => {
                            HttpResponse::build(StatusCode::from_u16(e.http_status).unwrap()).json(json!({
                                "code": e.code,
                                "reason": e.reason,
                                "message": e.message
                            }))
                        }
                    }
                    
                }
                None => {
                    HttpResponse::build(StatusCode::from_u16(ERR_NOT_FOUND.http_status).unwrap()).json(json!({
                        "code": ERR_NOT_FOUND.code,
                        "reason": ERR_NOT_FOUND.reason,
                        "message": format!("User '{}' not found.", &login.name)
                    }))
                }
            }
        }
        Err(e) => {
            HttpResponse::build(StatusCode::from_u16(e.http_status).unwrap()).json(json!({
                "code": e.code,
                "reason": e.reason,
                "message": e.message
            }))
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct RanklistQuery {
    #[serde(default)]
    pub scoring_rule: ScoringRule,
    pub tie_breaker: Option<TieBreaker>
}

#[get("/contests/{id}/ranklist")]
pub async fn get_contest_ranklist(req: HttpRequest, query: web::Query<RanklistQuery>, path: web::Path<i32>) -> impl Responder {
    let worker = req.app_data::<Arc<Worker>>().unwrap();
    match match path.into_inner() {
        0 => worker.global_ranklist(query.scoring_rule, query.tie_breaker),
        id => worker.contest_ranklist(id, query.scoring_rule, query.tie_breaker)
    } {
        Ok(ranklist) => {
            HttpResponse::Ok().json(ranklist)
        }
        Err(e) => {
            HttpResponse::build(StatusCode::from_u16(e.http_status).unwrap()).json(json!({
                "code": e.code,
                "reason": e.reason,
                "message": e.message
            }))
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ContestUpdate {
    pub id: Option<i32>,
    pub name: String,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub problem_ids: Vec<i32>,
    pub user_ids: Vec<i32>,
    pub submission_limit: i32
}

#[post("/contests")]
pub async fn post_contests(req: HttpRequest, update: web::Json<ContestUpdate>) -> impl Responder {
    if update.id == Some(0) {
        return HttpResponse::build(StatusCode::from_u16(ERR_INVALID_ARGUMENT.http_status).unwrap()).json(json!({
            "code": ERR_INVALID_ARGUMENT.code,
            "reason": ERR_INVALID_ARGUMENT.reason,
            "message": "Invalid contest id"
        }));
    }
    let worker = req.app_data::<Arc<Worker>>().unwrap();
    let contest;
    match match update.id {
        Some(id) => {
            contest = Contest {
                id,
                name: update.name.clone(),
                from: update.from,
                to: update.to,
                problem_ids: update.problem_ids.clone(),
                user_ids: update.user_ids.clone(),
                submission_limit: update.submission_limit,
            };
            worker.database().put_contest(&contest)
        },
        None => {
            contest = Contest {
                id: worker.next_contest_id(),
                name: update.name.clone(),
                from: update.from,
                to: update.to,
                problem_ids: update.problem_ids.clone(),
                user_ids: update.user_ids.clone(),
                submission_limit: update.submission_limit,
            };
            worker.database().put_contest(&contest)
        }
    } {
        Ok(_) => {
            HttpResponse::Ok().json(contest)
        }
        Err(e) => {
            HttpResponse::build(StatusCode::from_u16(e.http_status).unwrap()).json(json!({
                "code": e.code,
                "reason": e.reason,
                "message": e.message
            }))
        }
    }
}

#[get("/contests")]
pub async fn get_contests(req: HttpRequest) -> impl Responder {
    let worker = req.app_data::<Arc<Worker>>().unwrap();
    match worker.database().list_contests() {
        Ok(mut contests) => {
            contests.sort_by_key(|contest| contest.id);
            HttpResponse::Ok().json(contests)
        }
        Err(e) => {
            HttpResponse::build(StatusCode::from_u16(e.http_status).unwrap()).json(json!({
                "code": e.code,
                "reason": e.reason,
                "message": e.message
            }))
        }
    }
}

#[get("/contests/{id}")]
pub async fn get_contest_by_id(req: HttpRequest, path: web::Path<i32>) -> impl Responder {
    let id = path.into_inner();
    let worker = req.app_data::<Arc<Worker>>().unwrap();
    match worker.database().find_contest_by_id(id) {
        Ok(Some(contest)) => {
            HttpResponse::Ok().json(contest)
        }
        Ok(None) => {
            HttpResponse::build(StatusCode::from_u16(ERR_NOT_FOUND.http_status).unwrap()).json(json!({
                "code": ERR_NOT_FOUND.code,
                "reason": ERR_NOT_FOUND.reason,
                "message": format!("Contest {} not found.", id),
            }))
        }
        Err(e) => {
            HttpResponse::build(StatusCode::from_u16(e.http_status).unwrap()).json(json!({
                "code": e.code,
                "reason": e.reason,
                "message": e.message,
            }))
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProblemDisplay {
    pub id: i32,
    pub name: String,
    #[serde(rename = "type")]
    pub problem_type: ProblemType,
    #[serde(rename = "desc")]
    pub description: String,
    pub cases: usize,
    pub score: f32,
}

impl ProblemDisplay {
    fn from_problem(problem: &Problem) -> Self {
        Self {
            id: problem.id,
            name: problem.name.clone(),
            problem_type: problem.problem_type,
            description: problem.description.clone(),
            cases: problem.cases.len(),
            score: {
                let mut score = 0f32;
                for case in &problem.cases {
                    score += case.score;
                }
                score
            }
        }
    }
}

#[get("/contests/{id}/problems")]
pub async fn get_contest_problems(req: HttpRequest, path: web::Path<i32>) -> impl Responder {
    let id = path.into_inner();
    let worker = req.app_data::<Arc<Worker>>().unwrap();
    match match id {
        0 => Ok(worker.config.problems.iter().collect()),
        id => worker.get_contest_problems(id) 
    } {
        Ok(problems) => {
            let problems = problems.into_iter().map(ProblemDisplay::from_problem).collect::<Vec<_>>();
            HttpResponse::Ok().json(problems)
        }
        Err(e) => {
            HttpResponse::build(StatusCode::from_u16(e.http_status).unwrap()).json(json!({
                "code": e.code,
                "reason": e.reason,
                "message": e.message,
            }))
        }
    }
}

#[get("/problems/{id}")]
pub async fn get_problem_by_id(req: HttpRequest, path: web::Path<i32>) -> impl Responder {
    let id = path.into_inner();
    let worker = req.app_data::<Arc<Worker>>().unwrap();
    match worker.config.problems.iter().find(|problem| problem.id == id) {
        Some(problem) => {
            HttpResponse::Ok().json(ProblemDisplay::from_problem(problem))
        }
        None => {
            HttpResponse::build(StatusCode::from_u16(ERR_NOT_FOUND.http_status).unwrap()).json(json!({
                "code": ERR_NOT_FOUND.code,
                "reason": ERR_NOT_FOUND.reason,
                "message": format!("Problem {} not found.", id)
            }))
        }
    }
}
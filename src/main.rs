use actix_web::*;
use actix_files::Files;
use actix_cors::Cors;
use clap::Parser;
use models::Config;
use tokens::TokenFactory;
use worker::Worker;
use std::{fs::File, sync::Arc};
use server::*;

mod server;
mod database;
mod judge;
mod models;
mod serde_helper;
mod tokens;
mod worker;

#[derive(Parser, Debug)]
#[command(name = "oj")]
pub struct CommandLine {
    #[arg(long, short)]
    pub config: String,
    #[arg(long, short)]
    pub flush_data: bool,
    #[arg(long, short)]
    pub auth: bool
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let cl = CommandLine::parse();
    let config: Config = serde_json::from_reader(File::open(cl.config)?).unwrap();
    let worker = Arc::new(Worker::new(config, cl.flush_data).unwrap());
    worker.start();
    let worker_clone = worker.clone();
    HttpServer::new(move || {
        App::new()
            .app_data(worker_clone.clone())
            .app_data(TokenFactory::new())
            .app_data(cl.auth)
            .wrap(middleware::Logger::default())
            .wrap(Cors::permissive())
            .service(greet)
            .service(exit)
            .service(post_jobs)
            .service(get_jobs)
            .service(get_job_by_id)
            .service(rerun_job)
            .service(cancel_job)
            .service(get_users)
            .service(post_users)
            .service(login)
            .service(post_contests)
            .service(get_contests)
            .service(get_contest_by_id)
            .service(get_contest_ranklist)
            .service(get_contest_problems)
            .service(get_problem_by_id)
            .service(Files::new("/", "./frontend/.output/public").index_file("index.html"))
    })
    .bind((
        worker.config.server.bind_address.as_str(), 
        worker.config.server.bind_port
    ))?
    .run()
    .await
}


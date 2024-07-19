use std::fs::remove_file;

use redb::TableDefinition;
use crate::models::*;

pub struct Database {
    inner: redb::Database
}

const JOBS: TableDefinition<i32, Vec<u8>> = TableDefinition::new("jobs");
const CONTESTS: TableDefinition<i32, Vec<u8>> = TableDefinition::new("contests");
const USERS: TableDefinition<i32, Vec<u8>> = TableDefinition::new("users");

impl Database {
    pub fn new(flush_data: bool) -> Result<Self> {
        if flush_data {
            remove_file("storage.redb")?;
        }
        let db = redb::Database::create("storage.redb")?;
        let write_txn = db.begin_write()?;
        {
            write_txn.open_table(JOBS)?;
            write_txn.open_table(CONTESTS)?;
            let mut users = write_txn.open_table(USERS)?;
            users.insert(0, serde_json::to_vec(&User {
                id: 0,
                name: "root".into()
            })?)?;
        }
        write_txn.commit()?;
        Ok(Self {
            inner: db
        })
    }
    pub fn largest_job_id(&self) -> Result<i32> {
        let read_txn = self.inner.begin_read()?;
        {
            let jobs = read_txn.open_table(JOBS)?;
            let mut max = -1;
            for kv in jobs.range::<i32>(..)? {
                match kv {
                    Ok(kv) => if kv.0.value() > max { max = kv.0.value() },
                    Err(_) => break
                }
            }
            Ok(max)
        }
    }
    pub fn put_job(&self, job: &Job) -> Result<()> {
        let write_txn = self.inner.begin_write()?;
        {
            let mut jobs = write_txn.open_table(JOBS)?;
            jobs.insert(job.id, serde_json::to_vec(job)?)?;
        }
        write_txn.commit()?;
        Ok(())
    }
    pub fn find_job<F>(&self, mut predicate: F) -> Result<Option<Job>> where F: FnMut(&Job) -> Result<bool> {
        let read_txn = self.inner.begin_read()?;
        {
            let jobs = read_txn.open_table(JOBS)?;
            for kv in jobs.range::<i32>(..)? {
                if kv.is_err() { break }
                let kv = kv.unwrap();
                let job: Job = serde_json::from_slice(&kv.1.value())?;
                if predicate(&job)? {
                    return Ok(Some(job));
                }
            }
        }
        Ok(None)
    }
    pub fn find_jobs<F>(&self, mut predicate: F) -> Result<Vec<Job>> where F: FnMut(&Job) -> Result<bool> {
        let read_txn = self.inner.begin_read()?;
        let mut result = vec![];
        {
            let jobs = read_txn.open_table(JOBS)?;
            for kv in jobs.range::<i32>(..)? {
                if kv.is_err() { break }
                let kv = kv.unwrap();
                let job: Job = serde_json::from_slice(&kv.1.value())?;
                if predicate(&job)? {
                    result.push(job);
                }
            }
        }
        Ok(result)
    }
    pub fn largest_user_id(&self) -> Result<i32> {
        let read_txn = self.inner.begin_read()?;
        {
            let users = read_txn.open_table(USERS)?;
            let mut max = -1;
            for kv in users.range::<i32>(..)? {
                match kv {
                    Ok(kv) => if kv.0.value() > max { max = kv.0.value() },
                    Err(_) => break
                }
            }
            Ok(max)
        }
    }
    pub fn user_exists(&self, id: i32) -> Result<bool> {
        let read_txn = self.inner.begin_read()?;
        {
            let users = read_txn.open_table(USERS)?;
            Ok(users.get(id)?.is_some())
        }
    }
    pub fn find_user_by_name(&self, name: &str) -> Result<Option<User>> {
        let read_txn = self.inner.begin_read()?;
        {
            let jobs = read_txn.open_table(USERS)?;
            for kv in jobs.range::<i32>(..)? {
                if kv.is_err() { break }
                let kv = kv.unwrap();
                let user: User = serde_json::from_slice(&kv.1.value())?;
                if user.name == name {
                    return Ok(Some(user));
                }
            }
        }
        Ok(None)
    }
    pub fn list_users(&self) -> Result<Vec<User>> {
        let read_txn = self.inner.begin_read()?;
        let mut result = vec![];
        {
            let users = read_txn.open_table(USERS)?;
            for kv in users.range::<i32>(..)? {
                if kv.is_err() { break }
                let kv = kv.unwrap();
                let user: User = serde_json::from_slice(&kv.1.value())?;
                result.push(user);
            }
        }
        Ok(result)
    }
    pub fn find_users<F>(&self, mut predicate: F) -> Result<Vec<User>> where F: FnMut(&User) -> bool {
        let read_txn = self.inner.begin_read()?;
        let mut result = vec![];
        {
            let users = read_txn.open_table(USERS)?;
            for kv in users.range::<i32>(..)? {
                if kv.is_err() { break }
                let kv = kv.unwrap();
                let user: User = serde_json::from_slice(&kv.1.value())?;
                if predicate(&user) {
                    result.push(user);
                }
            }
        }
        Ok(result)
    }
    pub fn put_user(&self, user: &User) -> Result<()> {
        let write_txn = self.inner.begin_write()?;
        {
            let mut users = write_txn.open_table(USERS)?;
            users.insert(user.id, serde_json::to_vec(user)?)?;
        }
        write_txn.commit()?;
        Ok(())
    }
    pub fn largest_contest_id(&self) -> Result<i32> {
        let read_txn = self.inner.begin_read()?;
        {
            let users = read_txn.open_table(CONTESTS)?;
            let mut max = 0;
            for kv in users.range::<i32>(..)? {
                match kv {
                    Ok(kv) => if kv.0.value() > max { max = kv.0.value() },
                    Err(_) => break
                }
            }
            Ok(max)
        }
    }
    pub fn list_contests(&self) -> Result<Vec<Contest>> {
        let read_txn = self.inner.begin_read()?;
        let mut result = vec![];
        {
            let contests = read_txn.open_table(CONTESTS)?;
            for kv in contests.range::<i32>(..)? {
                if kv.is_err() { break; }
                let kv = kv.unwrap();
                let contest: Contest = serde_json::from_slice(&kv.1.value())?;
                result.push(contest);
            }
        }
        Ok(result)
    }
    pub fn put_contest(&self, contest: &Contest) -> Result<()> {
        let write_txn = self.inner.begin_write()?;
        {
            let mut contests = write_txn.open_table(CONTESTS)?;
            contests.insert(contest.id, serde_json::to_vec(contest)?)?;
        }
        write_txn.commit()?;
        Ok(())
    }
    pub fn find_contest_by_id(&self, id: i32) -> Result<Option<Contest>> {
        let read_txn = self.inner.begin_read()?;
        {
            let contests = read_txn.open_table(CONTESTS)?;
            match contests.get(id)? {
                Some(v) => Ok(Some(serde_json::from_slice::<Contest>(&v.value())?)),
                None => Ok(None)
            }
        }
    }
}
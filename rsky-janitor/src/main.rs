use dotenvy::dotenv;
use chrono::{Utc};
use cron::Schedule;
use std::str::FromStr;
use std::{env, thread};
use postgres::{Client, NoTls};

fn main() {
    eprintln!("Starting Janitor");
    dotenv().ok();
    let cron_schedule = env::var("CRON_SCHEDULE").unwrap_or("0 0 0 * * * *".to_string());
    let username = env::var("USERNAME").expect("Missing username");
    let password = env::var("PASSWORD").expect("Missing password");
    let schedule = Schedule::from_str(cron_schedule.as_str()).expect("Failed to parse CRON expression");

    loop {
        eprintln!("Looping");
        let now = Utc::now();
        if let Some(next) = schedule.upcoming(Utc).take(1).next() {
            let until_next = next - now;
            thread::sleep(until_next.to_std().unwrap());
            clean_db(username.as_str(), password.as_str());
        }
    }
}

fn clean_db(username: &str, password: &str) {
    let mut client = Client::connect(format!("host={username} user={password}").as_str(), NoTls).expect("Unable to connect");
    client.execute("DELETE FROM post where date(\"indexedAt\") < now() - interval '1 day'", &[]).expect("Failed to clean posts");
    client.execute("DELETE FROM repost where date(\"indexedAt\") < now() - interval '1 day'", &[]).expect("Failed to clean reposts");
    client.execute("DELETE FROM \"like\" where date(\"indexedAt\") < now() - interval '1 day'", &[]).expect("Failed to clean likes");
}
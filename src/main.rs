use std::{time::{Instant}, sync::{Arc, Mutex}};

use chrono::Datelike;

#[derive(serde::Deserialize, Debug)]
struct Logs {
    messages: Vec<Message>
}

#[allow(non_snake_case)]
#[derive(serde::Deserialize, Debug)]
struct Message {
    displayName: String,
    text: String,
    tags: MessageTags
}

#[derive(serde::Deserialize, Debug)]
struct MessageTags {
    #[serde(rename = "tmi-sent-ts")]
    tmi_sent_ts: String
}

#[derive(serde::Deserialize)]
struct SevenTVResponse(Vec<Emote>);

#[derive(serde::Deserialize)]
struct Emote {
    name: String
}

fn personal_logs(channel: &String, username: &String) -> Logs {
    let start = Instant::now();
    let logs: Logs = reqwest::blocking::get(
        format!("https://logs.ivr.fi/channel/{}/user/{}?json", channel, username)
    ).unwrap().json().unwrap();
    println!("ivr log query took {}ms", start.elapsed().as_millis());
    return logs;
}

#[allow(dead_code)]
fn channel_logs(channel: &String) -> Logs {
    let start = Instant::now();
    let mut logs = Logs { messages: Vec::new() };

    let date = chrono::Utc::now();


    for i in 1..=date.day() {
        let daily = reqwest::blocking::get(
            format!("https://logs.ivr.fi/channel/{}/2022/12/{}?json", channel, i)
        );
        if let Ok(req) = daily {
            if let Ok(msgs) = req.json::<Logs>() {
                logs.messages.extend(msgs.messages);
            }
        }
    }
    println!("ivr log query took {}ms", start.elapsed().as_millis());
    return logs;
}

fn channel_logs_mt(channel: &String) -> Logs {
    let start = Instant::now();
    let logs = Arc::new(Mutex::new(Logs { messages: Vec::new() }));

    let date = chrono::Utc::now();
    let month = date.month();


    let mut handles: Vec<std::thread::JoinHandle<()>> = Vec::new();
    for i in 1..=date.day() {
        let channel_name = channel.clone();
        let logs_local = logs.clone();
        handles.push(
            std::thread::spawn(move || {
                for _ in 0..5 {
                    let daily = reqwest::blocking::Client::builder().timeout(None).build().unwrap().get(
                        format!("https://logs.ivr.fi/channel/{}/2022/{}/{}?json", channel_name, month, i)
                    );
                    match daily.send() {
                        Ok(req) => { match serde_json::from_str::<Logs>(&req.text().unwrap()) {
                            Ok(msgs) => {
                                logs_local.lock().unwrap().messages.extend(msgs.messages);
                                break;
                            },
                            Err(e) => {
                                println!("failed parsing json for day {}: {}", i, e);
                                std::thread::sleep(std::time::Duration::from_secs(25));
                            },
                        }},
                        Err(e) => {
                            println!("failed requesting json for day {}: {}", i, e);
                            std::thread::sleep(std::time::Duration::from_secs(25));
                        },
                    }
                }
            })
        );
    }

    handles.into_iter().for_each(|h| h.join().unwrap());

    println!("ivr log query took {}ms", start.elapsed().as_millis());
    return Arc::try_unwrap(logs).unwrap().into_inner().unwrap();
}

#[allow(dead_code)]
fn top_chatter(channel: &String, len: usize) {
    let start = Instant::now();

    let date = chrono::Utc::now() - chrono::Duration::days(1);

    let daily = reqwest::blocking::Client::builder()
        .timeout(None)
        .build()
        .unwrap()
        .get(
            format!("https://logs.ivr.fi/channel/{}/{}?json=", channel, date.format("%Y/%m/%d"))
        );

    let logs = match daily.send() {
        Ok(req) => {
            println!("{}", req.url());
            // let headers = req.headers();
            // for i in headers.iter() {
            //     println!("{}: {:?}", i.0, i.1);
            // }
            let slice = req.bytes().unwrap();
            match serde_json::from_slice::<Logs>(&slice) {
                Ok(msgs) => {
                    msgs
                },
                Err(e) => {
                    println!("failed parsing json for day {}: {}", date.day(), e);
                    panic!();
                }
        }},
        Err(e) => panic!("failed requesting json for day {}: {}", date.day(), e),
    };
    println!("ivr log query took {}ms", start.elapsed().as_millis());

    let mut counter: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

    for i in logs.messages {
        match counter.get_mut(&i.displayName) {
            Some(count) => *count += 1,
            None => {counter.insert(i.displayName, 1);},
        };
    }

    let mut sorted: Vec<(String, u32)> = counter.drain().collect();
    sorted.sort_by_key(|m| m.1);

    for i in 0..len {
        if let Some(index) = sorted.len().checked_sub(1 + i) {
            if let Some(item) = sorted.get(index) {
                println!("{} chatted {} times", item.0, item.1);
            } else {
                break;
            }
        } else {
            break;
        }
    }
}

fn top_past_24h(channel: &String, len: usize) {
    let start = Instant::now();

    let date = chrono::Utc::now();
    let yesterdate = date - chrono::Duration::days(1);

    let today = reqwest::blocking::Client::builder()
        .timeout(None)
        .build()
        .unwrap()
        .get(
            format!("https://logs.ivr.fi/channel/{}/{}?json=", channel, date.format("%Y/%m/%d"))
        );

    let yesterday = reqwest::blocking::Client::builder()
        .timeout(None)
        .build()
        .unwrap()
        .get(
            format!("https://logs.ivr.fi/channel/{}/{}?json=", channel, yesterdate.format("%Y/%m/%d"))
        );

    let today_logs = match today.send() {
        Ok(req) => {
            #[cfg(debug_assertions)]
            println!("sending request for: {}", req.url());
            // let headers = req.headers();
            // for i in headers.iter() {
            //     println!("{}: {:?}", i.0, i.1);
            // }
            let slice = req.bytes().unwrap();
            match serde_json::from_slice::<Logs>(&slice) {
                Ok(msgs) => {
                    msgs
                },
                Err(e) => {
                    panic!("failed parsing json for day {}: {}", date.day(), e);
                }
        }},
        Err(e) => panic!("failed requesting json for day {}: {}", date.day(), e),
    };

    let yesterday_logs = match yesterday.send() {
        Ok(req) => {
            #[cfg(debug_assertions)]
            println!("sending request for: {}", req.url());
            // let headers = req.headers();
            // for i in headers.iter() {
            //     println!("{}: {:?}", i.0, i.1);
            // }
            let slice = req.bytes().unwrap();
            match serde_json::from_slice::<Logs>(&slice) {
                Ok(msgs) => {
                    msgs
                },
                Err(e) => {
                    println!("failed parsing json for day {}: {}", date.day(), e);
                    panic!();
                }
        }},
        Err(e) => panic!("failed requesting json for day {}: {}", date.day(), e),
    };
    #[cfg(debug_assertions)]
    println!("ivr log queries took {}ms\n", start.elapsed().as_millis());

    let mut counter: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

    for i in today_logs.messages {
        match counter.get_mut(&i.displayName) {
            Some(count) => *count += 1,
            None => {counter.insert(i.displayName, 1);},
        };
    }

    for i in yesterday_logs.messages {
        let timestamp = i.tags.tmi_sent_ts.parse::<i64>().unwrap();
        if yesterdate.timestamp_millis() < timestamp {
            match counter.get_mut(&i.displayName) {
                Some(count) => *count += 1,
                None => {counter.insert(i.displayName, 1);},
            };
        }

    }

    let mut sorted: Vec<(String, u32)> = counter.drain().collect();
    sorted.sort_by_key(|m| m.1);

    for i in 0..len {
        if let Some(index) = sorted.len().checked_sub(1 + i) {
            if let Some(item) = sorted.get(index) {
                println!("{} chatted {} times", item.0, item.1);
            } else {
                break;
            }
        } else {
            break;
        }
    }
}

fn top_emotes(channel:String, username: Option<String>, len: usize) {
    let logs = match username {
        Some(un) => personal_logs(&channel, &un),
        None => channel_logs_mt(&channel),
    };
    let start = Instant::now();
    let seventv_channel_emotes: SevenTVResponse = reqwest::blocking::get(
        format!("https://api.7tv.app/v2/users/{}/emotes", channel)
    ).unwrap().json().unwrap();
    println!("7TV channel emote query took {}ms", start.elapsed().as_millis());

    let start = Instant::now();
    let seventv_global_emotes: SevenTVResponse = reqwest::blocking::get(
        "https://api.7tv.app/v2/emotes/global"
    ).unwrap().json().unwrap();
    println!("7TV global emote query took {}ms\n", start.elapsed().as_millis());

    let mut emotes: Vec<String> = Vec::new();

    emotes.extend(seventv_channel_emotes.0.into_iter().map(|e| e.name));
    emotes.extend(seventv_global_emotes.0.into_iter().map(|e| e.name));

    let mut counter: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

    for (i, msg) in logs.messages.iter().enumerate() {
        if i % logs.messages.len() / 100 == 0 || i == logs.messages.len() - 1 {
            print!("\x1B[2K\x1b[1G");
            print!("processed {} out of {} messages", i + 1, logs.messages.len());
        }
        let words = msg.text.split_whitespace();
        for word in words {
            if let Some(count) = counter.get(word) {
                if emotes.contains(&word.to_owned()) {
                    counter.insert(word.to_owned(), *count + 1);
                }
            } else {
                if emotes.contains(&word.to_owned()) {
                    counter.insert(word.to_owned(), 1);
                }
            }
        }
    }

    print!("\n\n");

    let mut sorted: Vec<(String, u32)> = Vec::new();

    for (key, val) in counter.drain() {
        sorted.push((key, val));
    }

    sorted.sort_by_key(|v| v.1);

    for i in 0..len {
        if let Some(index) = sorted.len().checked_sub(1 + i) {
            if let Some(item) = sorted.get(index) {
                println!("{} was used {} times", item.0, item.1);
            } else {
                break;
            }
        } else {
            break;
        }
    }
}

struct Params {
    channel: Option<String>,
    username: Option<String>,
    top: bool,
    leaderboard_len: usize
}

impl Params {
    fn new() -> Self {
        Params {
            channel: None,
            username: None,
            top: false,
            leaderboard_len: 10
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut opts = Params::new();
    let mut i: usize = 1;
    loop {
        match args.get(i) {
            Some(arg) => {
                match arg.as_str() {
                    "--channel" => {
                        match args.get(i+1) {
                            Some(opt) => opts.channel = Some(opt.clone()),
                            None => panic!("option requires a value afterwards"),
                        }
                        i += 2;
                    },
                    "--user" => {
                        match args.get(i+1) {
                            Some(opt) => opts.username = Some(opt.clone()),
                            None => panic!("option requires a value afterwards"),
                        }
                        i += 2;
                    },
                    "--length" => {
                        match args.get(i+1) {
                            Some(opt) => opts.leaderboard_len = opt.parse().expect("length must be a number"),
                            None => panic!("option requires a value afterwards"),
                        }
                        i += 2;
                    },
                    "--top" => {
                        opts.top = true;
                        i += 1
                    }
                    _ => panic!("argument not recognized!")
                };
            },
            None => break,
        };
    }
    drop(i);

    let channel = match opts.channel {
        Some(s) => s,
        None => panic!("channel name required"),
    };
    if opts.top {
        top_past_24h(&channel, opts.leaderboard_len);
    } else {
        top_emotes(channel, opts.username, opts.leaderboard_len);
    }


}

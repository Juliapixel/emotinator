use std::time::Instant;

#[derive(serde::Deserialize)]
struct Logs {
    messages: Vec<Message>
}

#[derive(serde::Deserialize)]
struct Message {
    text: String
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

fn channel_logs(channel: &String) -> Logs {
    let start = Instant::now();
    let mut logs = Logs { messages: Vec::new() };
    for i in 0..31 {
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

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let channel = args.get(1).expect("no channel given");
    let username = args.get(2);

    let logs = match username {
        Some(un) => personal_logs(channel, un),
        None => channel_logs(channel),
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

    for msg in logs.messages {
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

    let mut sorted: Vec<(String, u32)> = Vec::new();

    for (key, val) in counter.drain() {
        sorted.push((key, val));
    }

    sorted.sort_by_key(|v| v.1);

    for i in 0..25 {
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

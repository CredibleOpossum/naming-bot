use std::fs::File;
use std::io::Write;
use std::time::Duration;
use std::{env, fs};

use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

use std::sync::Mutex;
use std::thread;

static NAME_DATA: Mutex<NameData> = Mutex::new(NameData { num: 0 });

struct NameData {
    num: u32,
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == "!ping" {
            if let Err(why) = msg.channel_id.say(&ctx.http, "Pong!").await {
                println!("Error sending message: {:?}", why);
            }
        }
        if msg.content == "!name" {
            // Get and increment number
            let number;
            {
                let mut data = NAME_DATA.lock().unwrap();
                number = data.num;
                data.num += 1;
            }

            let member = &msg
                .guild_id
                .unwrap()
                .member(&ctx, msg.author)
                .await
                .unwrap();
            let name = member.display_name().to_string();

            let number_text = format!("[{number}]");

            let full_nickname = format!("{name} {number_text}");

            // Set nickname
            msg.guild_id
                .unwrap()
                .member(&ctx, 730885117656039466)
                .await
                .unwrap()
                .edit(&ctx, |m| m.nickname(full_nickname))
                .await
                .unwrap();
        }
    }

    async fn guild_member_addition(&self, ctx: Context, guild_id: serenity::model::guild::Member) {
        let number;
        {
            let mut data = NAME_DATA.lock().unwrap();
            number = data.num;
            data.num += 1;
        }

        let name = guild_id.display_name();
        let number_text = format!("[{number}]");

        let full_nickname = format!("{name} {number_text}");

        // Set nickname
        guild_id
            .edit(&ctx, |m| m.nickname(full_nickname))
            .await
            .unwrap();
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    // Load number if exists
    let file_path = "num.txt";
    if fs::metadata(file_path).is_ok() {
        let num = fs::read_to_string(file_path).unwrap().parse().unwrap();
        {
            NAME_DATA.lock().unwrap().num = num
        }
    }

    // Spawn saving thread
    thread::spawn(move || loop {
        let num;
        thread::sleep(Duration::new(5, 0));
        {
            num = NAME_DATA.lock().unwrap().num;
        }
        let mut data_file = File::create(file_path).expect("creation failed");
        data_file
            .write_all(num.to_string().as_bytes())
            .expect("write failed");
    });

    // Do boilerplate stuff
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}

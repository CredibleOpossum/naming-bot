use std::fs::File;
use std::io::Write;
use std::time::Duration;
use std::{env, error, fs};

use serenity::all::{GuildId, Reaction, ReactionType, Role, RoleId};
use serenity::async_trait;
use serenity::builder::EditMember;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

use std::sync::Mutex;
use std::thread;

static NAME_DATA: Mutex<NameData> = Mutex::new(NameData { num: 0 });
const EMOJI_LIST: [&str; 9] = ["😁", "👍", "🔥", "💀", "😟", "😎", "🦀", "🤑", "❤️"];
const SELF_ID: u64 = 1152776929024409621;
struct NameData {
    num: u32,
}

async fn get_askme_roles(ctx: &Context, guild: &GuildId) -> Vec<(RoleId, Role)> {
    let mut role_list = Vec::new();
    let roles = guild.roles(&ctx).await.unwrap();
    for role in roles {
        if role.1.name.starts_with("AskMeAbout_") {
            role_list.push(role);
        }
    }
    role_list.sort();
    role_list
}

async fn handle_message(ctx: Context, msg: Message) -> Result<(), Box<dyn error::Error>> {
    if msg.content == "!ping" {
        if let Err(why) = msg.channel_id.say(&ctx.http, "Pong!").await {
            println!("Error sending message: {:?}", why);
        }
    }
    if msg.content == "!name" {
        let guild = msg.guild_id.unwrap();
        // Get and increment number
        let number;
        {
            let mut data = NAME_DATA.lock()?;
            number = data.num;
            data.num += 1;
        }

        let member = guild.member(&ctx, &msg.author).await?;
        let name = &member.display_name();

        let number_text = format!("[{number}]");

        let full_nickname = format!("{name} {number_text}");

        // Set nickname
        let builder = EditMember::new().nickname(full_nickname);
        guild
            .edit_member(&ctx.http, &msg.author.id, builder)
            .await?;
    }
    if msg.content == "!roles" {
        let mut role_message = "React to get role".to_string();
        let role_list = get_askme_roles(&ctx, &msg.guild_id.unwrap()).await;
        let role_range = 0..role_list.len();
        for index in role_range.clone() {
            role_message = format!(
                "{}\n{} = {}",
                role_message, EMOJI_LIST[index], role_list[index].1.name
            )
        }
        let message = msg.channel_id.say(&ctx.http, role_message).await?;

        for emoji in role_range {
            message
                .react(&ctx, ReactionType::Unicode(EMOJI_LIST[emoji].into()))
                .await?;
        }
    }
    Ok(())
}

async fn reaction_handle(
    ctx: Context,
    reaction: Reaction,
    is_being_added: bool,
) -> Result<(), Box<dyn error::Error>> {
    if u64::from(reaction.message(&ctx).await?.author.id.0) != SELF_ID {
        return Ok(()); // We aren't being reacted to, lets just ignore
    }
    if reaction.user(&ctx).await?.bot {
        return Ok(()); // A bot is reacting to us, lets ignore it
    }
    if let Some(index) = EMOJI_LIST
        .iter()
        .position(|&x| x == reaction.emoji.to_string())
    {
        let role_list =
            get_askme_roles(&ctx, &reaction.guild_id.ok_or("failed to find guild")?).await;
        let role = &role_list[index];
        let mut member = GuildId(reaction.guild_id.ok_or("failed to find guild")?.0)
            .member(&ctx, reaction.user_id.ok_or("failed to find user")?.0)
            .await
            .unwrap();

        if is_being_added {
            member.add_role(&ctx, role.0).await?;
        } else {
            member.remove_role(&ctx, role.0).await?;
        }
    }
    Ok(())
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if let Err(why) = handle_message(ctx, msg).await {
            eprintln!("{}", why);
        }
    }

    async fn guild_member_addition(&self, ctx: Context, member: serenity::model::guild::Member) {
        let number;
        {
            let mut data = NAME_DATA.lock().unwrap();
            number = data.num;
            data.num += 1;
        }

        let name = member.display_name();
        let number_text = format!("[{number}]");

        let full_nickname = format!("{name} {number_text}");

        // Set nickname
        let builder = EditMember::new().nickname(full_nickname);
        let _ = member
            .guild_id
            .edit_member(&ctx.http, member, builder)
            .await;
    }

    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        if let Err(why) = reaction_handle(ctx, reaction, true).await {
            eprintln!("{}", why);
        };
    }

    async fn reaction_remove(&self, ctx: Context, reaction: Reaction) {
        if let Err(why) = reaction_handle(ctx, reaction, false).await {
            eprintln!("{}", why);
        };
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
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MESSAGE_REACTIONS
        | GatewayIntents::GUILD_MEMBERS;
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}

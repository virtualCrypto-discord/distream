use std::env;
use serenity::framework::StandardFramework;
use songbird::{Config, SerenityInit, EventContext, Event, EventHandler as VoiceEventHandler, CoreEvent};
use songbird::driver::DecodeMode;
use serenity::{Client, async_trait};
use serenity::prelude::{EventHandler, Context};
use serenity::model::prelude::{Ready, GuildId, Message};
use serenity::framework::standard::{Args, CommandResult};
use serenity::framework::standard::macros::{group, command};

mod bot;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

struct Receiver {
    guild_id: GuildId
}

impl Receiver {
    pub fn new(guild_id: GuildId) -> Self {
        // You can manage state here, such as a buffer of audio packet bytes so
        // you can later store them in intervals.
        Self {
            guild_id
        }
    }
}

#[async_trait]
impl VoiceEventHandler for Receiver {
    #[allow(unused_variables)]
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        use EventContext as Ctx;

        None
    }
}

#[group]
#[commands(join, leave)]
struct General;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let token = env::var("DISCORD_BOT_TOKEN")
        .expect("Expected a token in the environment");

    let framework = StandardFramework::new()
        .configure(|c| c.prefix("!"))
        .group(&GENERAL_GROUP);

    let songbird_config = Config::default()
        .decode_mode(DecodeMode::Decrypt);

    let mut client = Client::builder(&token)
        .event_handler(Handler)
        .framework(framework)
        .register_songbird_from_config(songbird_config)
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}

#[command]
#[only_in(guilds)]
async fn join(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let ch = msg.guild(&ctx.cache).await.unwrap().voice_states.get(&msg.author.id).and_then(|state| state.channel_id);
    match ch {
        Some(channel_id) => {
            let manager = songbird::get(ctx).await
                .expect("Songbird Voice client placed in at initialisation.").clone();

            let (handler_lock, conn_result) = manager.join(msg.guild_id.unwrap(), channel_id).await;

            if let Ok(_) = conn_result {
                // NOTE: this skips listening for the actual connection result.
                let mut handler = handler_lock.lock().await;

                handler.add_global_event(
                    CoreEvent::SpeakingStateUpdate.into(),
                    Receiver::new(msg.guild_id.unwrap())
                );

                handler.add_global_event(
                    CoreEvent::SpeakingUpdate.into(),
                    Receiver::new(msg.guild_id.unwrap())
                );

                handler.add_global_event(
                    CoreEvent::VoicePacket.into(),
                    Receiver::new(msg.guild_id.unwrap())
                );

                handler.add_global_event(
                    CoreEvent::RtcpPacket.into(),
                    Receiver::new(msg.guild_id.unwrap())
                );

                handler.add_global_event(
                    CoreEvent::ClientConnect.into(),
                    Receiver::new(msg.guild_id.unwrap())
                );

                handler.add_global_event(
                    CoreEvent::ClientDisconnect.into(),
                    Receiver::new(msg.guild_id.unwrap())
                );
            }
        }
        None => {
            let _ = msg.channel_id.say(&ctx.http, "you should connect voice channel.").await;
        }
    }
    Ok(())
}

#[command]
#[only_in(guilds)]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx).await
        .expect("Songbird Voice client placed in at initialisation.").clone();
    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        if let Err(e) = manager.remove(guild_id).await {
            let _ = msg.channel_id.say(&ctx.http, format!("Failed: {:?}", e)).await;
        }

        let _ = msg.channel_id.say(&ctx.http,"Left voice channel").await;
    } else {
        let _ = msg.reply(ctx, "Not in a voice channel").await;
    }

    Ok(())
}

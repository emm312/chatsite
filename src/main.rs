#[macro_use] extern crate rocket;

use rocket::{
    tokio::sync::broadcast::
    {
        channel, 
        Sender, error::RecvError
    }, 
    serde::{
        Serialize, 
        Deserialize
    }, 
    State, 
    response::stream::{
        EventStream, 
        Event
    }, 
    Shutdown, 
    fs::{
        FileServer, 
        relative
    }
};
use rocket::form::Form;
use rocket::tokio::select;

#[derive(Debug, Clone, FromForm, Serialize, Deserialize)]
#[serde(crate="rocket::serde")]
struct Message {
    #[field(validate = len(..30))] // roomname can only be 29 chars long
    pub room: String,
    #[field(validate = len(..20))] // username can only be 29 chars long
    pub username: String,
    pub message: String,
}

#[post("/message", data="<form>")]
fn post(form: Form<Message>, queue: &State<Sender<Message>>) {
    // fails if there is no active subscribers. thats OK
    let _res = queue.send(form.into_inner());
}

#[get("/events")]
async fn events(queue: &State<Sender<Message>>, mut end: Shutdown) -> EventStream![] {
    let mut rx = queue.subscribe();
    EventStream! {
        loop {
            let msg = select! {
                msg = rx.recv() => match msg {
                    Ok(msg) => msg,
                    Err(RecvError::Closed) => break, // when all connections are closed, break the loop
                    Err(RecvError::Lagged(_)) => continue, // if client is lagged too far behind, disconnect it
                },
                _ = &mut end => break, // when server is shutdown, break out of loop
            };

            yield Event::json(&msg); // if none of the break or continue statements are triggered, yield the message
        }
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .manage(channel::<Message>(1024).0)
        .mount("/", routes![post, events])
        .mount("/", FileServer::from(relative!("static")))
}
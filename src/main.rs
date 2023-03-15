#[macro_use] extern crate rocket;

#[cfg(test)] mod tests;

use rocket::{State, Shutdown};
use rocket::fs::{relative, FileServer};
use rocket::form::Form;
use rocket::response::stream::{EventStream, Event};
use rocket::serde::{Serialize, Deserialize};
use rocket::tokio::sync::broadcast::{channel, Sender, error::RecvError};
use rocket::tokio::select;


#[derive(Debug, Clone, FromForm, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq, UriDisplayQuery))]
#[serde(crate="rocket::serde")]
struct Message{
    #[field(validate = len(..30))]
    pub room: String,
    #[field(validate = len(..30))]
    pub username: String,
    pub message: String,
}

/// Return an infinite stream of server-send events. Each event is an message
/// pulled from a broadcast queue sent by the `post` handler.
#[get("/events")]
async fn events(queue: &State<Sender<Message>>, mut end: Shutdown) -> EventStream![]{
    let mut rx = queue.subscribe();
    EventStream! {
        loop {
            let msg = select! {
                msg = rx.recv() => match msg {
                    Ok(msg) => msg,
                    Err(RecvError::Closed) => break,
                    Err(RecvError::Lagged(_)) => continue,
                },
                _ = &mut end => break,
            };

            yield Event::json(&msg);
        }
    }
}

/// Recieve a message from a submission and broadcast it to any recievers.
#[post("/message", data = "<form>")]
fn post(form: Form<Message>, queue: &State<Sender<Message>>) {
    // A send 'fails' if there no active subscribers. That's okay.
    let _res = queue.send(form.into_inner());
}


#[launch]
fn rocket() -> _{
    rocket::build()
    .manage(channel::<Message>(1024).0)
    .mount("/", routes![post, events])
    .mount("/", FileServer::from(relative!("static")))
}

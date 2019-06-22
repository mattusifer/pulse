use actix_web::Responder;

pub fn index() -> impl Responder {
    "Hello world!"
}

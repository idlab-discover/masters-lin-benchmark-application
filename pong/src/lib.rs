wit_bindgen::generate!({ generate_all });
use exports::example::pong::pingpong::Guest;


struct Pong;

impl Guest for Pong {
    fn ping(input: String) -> String {
        input // Echo back
    }
}

export!(Pong);

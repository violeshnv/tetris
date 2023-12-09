mod game;

pub fn main() -> Result<(), String> {
    let mut game = game::Game::new();
    if let Err(s) = game.start() {
        println!("{}", s);
        return Err(s);
    }
    Ok(())
}

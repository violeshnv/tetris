mod game;

pub fn main() {
    let mut game = game::Game::new();
    game.start();
}

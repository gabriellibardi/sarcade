// Jogo controlado usado pelo ensaio reproduzível do RNF08.
import gleam/int
import sgleam/fill
import sgleam/image
import sgleam/mworld
import sgleam/pad

pub type Game {
  Game(player_1: Int, player_2: Int)
}

pub fn smain() {
  mworld.create(Game(0, 0), min: 2, max: 2, to_image: draw)
  |> mworld.keys_as_pad(pad.from_key)
  |> mworld.on_pad_down(score)
  |> mworld.stop_when(game_over)
}

fn game_over(game: Game) -> Bool {
  game.player_1 == 5 || game.player_2 == 5
}

fn score(game: Game, player: mworld.Player, event: pad.Event) -> Game {
  case event, mworld.player_index(player) {
    pad.ActionA, 1 -> Game(..game, player_1: game.player_1 + 1)
    pad.ActionA, 2 -> Game(..game, player_2: game.player_2 + 1)
    _, _ -> game
  }
}

fn draw(game: Game, player: mworld.Player) -> image.Image {
  let index = mworld.player_index(player)
  let #(mine, theirs, mine_color, theirs_color) = case index {
    1 -> #(game.player_1, game.player_2, fill.blue, fill.red)
    _ -> #(game.player_2, game.player_1, fill.red, fill.blue)
  }

  let score =
    image.above(
      image.text(
        "Jogador " <> int.to_string(index) <> ": " <> int.to_string(mine),
        30,
        fill.white,
      ),
      image.text("Adversario: " <> int.to_string(theirs), 24, fill.white),
    )

  score
  |> image.above(image.beside(
    image.rectangle(240, 100, mine_color),
    image.rectangle(240, 100, theirs_color),
  ))
  |> image.overlay(image.rectangle(640, 360, fill.black))
}

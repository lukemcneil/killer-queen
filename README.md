# Killer Queen


https://github.com/user-attachments/assets/11dd2800-690f-4fe5-892e-8723432e974f


This is a clone of the killer queen arcade game. Play either by
1. Cloning locally and running `cargo run --features bevy/dynamic_linking --release`
2. Go to https://lukemcneil.github.io/killer-queen/ (compiled to WebAssembly)

## How to Play

1. Connect as many gamepads as possible either through bluetooth or wired.
2. Join the game with R or L to join on the side you want. The first player to join on each side is the queen. Can also press select button to leave the game.
3. To start the game, both queens need to go over the start gate. This will remove the temporary blocking platform.
4. Controls once in the game-
    - left analog stick - move (you can wrap around the map where there is no wall)
    - south button (B on Switch) - jump as worker, fly as queen or fighter
    - as the queen you can hold down on the left analog stick to dive
5. How to win
    1. Economic - collect berries as workers and bring them back to your base.
    2. Ship - Ride the ship all the way to your side. Only workers can ride the ship, and they can jump off whenever they want.
    3. Military - kill the enemy queen 3 times. Only the queen or fighters can kill enemy queens.
6. Gates are scattered throughout the map. If a worker is holding a berry and stands in a gate for enough time, they become a fighter. They can now fly and fight just like the queen, but there deaths do not count towards a queen death leading to military victory. When a fighter dies, they respawn as a worker. Queens also have the unique ability to claim gates for their team by flying over them. A claimed gate can only be used by its team.
7. Queens and fighters kill workers of the other team if they touch them. If queens and fighters come in contact, then there are two cases-
    1. One player lands on top of the other - the player on bottom dies.
    2. The players hit each others sides - if one player is facing the others back, then the player with the back turned dies.

## MIDI Keyboard as Controller

You can also use a MIDI keyboard to serve as a controller for several players. On any octave, you can use C# or D# to join a team, C and D to move, and E to jump. You might have to tinker with `midi.rs` to correctly connect to the midi device. A keyboard turned turned out to be the perfect controller for this game, feeling like you are at the arcade playing on the actual cabinet.
![midi-controller](https://github.com/user-attachments/assets/07537be3-df56-483b-838c-9205abef87f6)

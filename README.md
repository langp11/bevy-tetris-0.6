# bevy-tetris-0.6

It's Tetris, made with [Bevy](https://github.com/bevyengine/bevy)!

Because clearly the world needs yet another Tetris clone.

Starting from the  [Bevy 0.4 example](https://github.com/8bit-pudding/bevy-tetris) but substantially re-written to meet Bevy 0.6 requirements and with additional features. Later updated to work with bevy 0.7.

More prosaically, when investigating Bevy 0.6 for a possible future project, there seemed to be a lack of examples the went beyond *here's a piece of code that works* to including the reasoning behind the code and the context in which it was required.

The code compiles debug builds without warnings in rust 1.60.0 and bevy 0.6.0, with one warning in release builds because of an unused variable.

`cargo clippy` is mostly happy, except for a few checks that I have explicitly allowed and commented.

There are lots of comments in the code that (hopefully) explain my specific reasons for various implementations.

## Commands

Standard operations:

* Move left: J, Left
* Move right: L, Right
* Down: K, Down
* Rotate clockwise: X
* Rotate anti-clockwise: Z
* Drop to bottom: Space
* Pause / unpause: P, Escape
* Restart: R
* Quit: Q

Additional operations available in debug builds:

Print a text version of the internal representation of the playing field: / (slash)

Check that the internal representation of the playing field agrees with the block entities: ' (apostrophe)

These extra operations have been left in to demonstrate (one way) how to conditionally compile code blocks.

## What is isn't

Doesn't implement all the _required_ rules from the [Tetris Guidelines](https://tetris.fandom.com/wiki/Tetris_Guideline), such as spins, holds, preview next piece etc.

I'm not suggesting that the methods used here are the best or only way to implement various features, they just worked for me.

## Features

Dyamically created and destroyed entities without loading assets from files.

Dynamic text elements that are modified and moved at runtime.

Simple keyboard event capture.

Window resizing event capture including modifying UI elements (the Text items).


## Application Design

### Entities

Primary entities are the blocks that comprise each tetromino. Once a tetromino has 'fallen' its individual blocks might be removed independantly as we clear a line, so we may as well start with just the blocks and use marker components to identify those that are part of the current tetromino.

A fallen tetromino is part of the `heap` of blocks at the bottom of the playing field.

We also need entities for the playing field and text UI elements.


### Components

Various shallow and marker components (ie structs with no content) to identify blocks in different states and text UI elements that may need moving and/or updating (eg the score).

### Resources

The 'soft drop' timer that moves the current tetromino down whether you like it or not. This interval reduces as you reach higher levels.

A larger structure holding the game configuration and current state (current level and score, is the game paused, is the current tetromino falling etc). For historical reasons, this is called `matrix` (and it does include a vector that represents the playing grid, which is sort of a matrix).

### Systems

The function name is in parentheses.

#### Startup (tetris_setup)

Creates the matrix resource based on the configuration constants.

Creates the static sprites for the playing field. These graphical elements are positioned relative to the centre of the window and automatically move as the window is resized (_automatically_ meaning Bevy does it for us).

#### Create new tetromino, update scores (spawn_current_tetromino)

Checks to see if any rows are full, in which case they are cleared and the heap blocks above moved down. 

This also updates the score which may update the level and increase the speed at which tetrominoes drop.

This function includes a debugging block that confirms that the matrix representation of the playing field agrees with the block entities. 

Randomly selects a new tetromino type and creates its blocks at the top of the playing field.

This system needs to be in a separate stage so that the block entity removal and creation does not interfere with the movement processes (the movement interferes with clearing rows but it is easier to move just one system to a seperate stage).


#### Movement (move_current_tetromino)
Responds to keyboard events to move the current tetromino around the playing field and various other operations.

Uses the soft drop timer to keep the current tetromino moving down.

Detects when the current tetromino has reached as low as it can, when it get moved to the heap. This triggers a new tetronimo creation or _game over_ if everything has goner horribly wrong.

#### Update block positions (update_block_sprites)

Any blocks that move within the playing field are marked with an `UpdateBlock` component. These can be either the current tetronimo which has been moved/rotated, or part of the heap that needs to move when a line is cleared.

The screen position of these sprites gets recalculated and the `UpdateBlock` component cleared.

These recalculations could happen in the systems that move the tetrominoes and rows, but Bevy favours more smaller systems over fewer larger ones - it means things can run in seperate threads, improving performance. 

#### Restart game (restart)

Resets the internal game state (`matrix`), removes any current block entities and triggers the game to start with a new tetromino.

#### Resize window (resize_window)

Text UI elements are positioned relative to the top left and do not automatically move when the screen size changes. This means that if we want some text to retain it's position relative to the sprites of the playing field, we have to remove and recreate them each time the screen size changes.

Some text elements have multiple sections (level and score), some have only one (status), so there are two different methods of creating the entities.

I couldn't find a way to change the position of a text element (a TextBundle) after it had been created, so removing and recreating them was my only solution. This may change in future versions of Bevy.

There is a resze event when the application starts, so we don't need to create our text UI elements anywhere else.

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::window::*;

use rand::{
    distributions::{Distribution, Standard},
    Rng,
};
use std::cmp::min;
use std::cmp::Ordering;
use std::time::Duration;

// ========================================
// Constants
// todo: most of these should be config parameters

/// A structure to contain configuration settings that vary between debug and release builds
struct Global {}

/// Debug
#[cfg(debug_assertions)]
impl Global {
    /// The space between blocks in pixels
    const BLOCK_SPACE: f32 = 1.0;

    /// The score background (RGBA)
    const SCOREFIELD_COLOR: (f32, f32, f32, f32) = (0.1, 0.0, 0.0, 0.5); 

    /// Should we display the grid lines?
    const DRAW_GRID: bool = true;

    /// Slow down the automatic drop by this factor
    const DROP_SPEED_FACTOR: f32 = 2.0; 

}

/// Release 
#[cfg(not(debug_assertions))]
impl Global {
    const BLOCK_SPACE: f32 = 0.0;
    const SCOREFIELD_COLOR: (f32, f32, f32, f32) = (0.1, 0.0, 0.0, 0.0); // Note 0.0 alpha = clear
    const DRAW_GRID: bool = false;
    const DROP_SPEED_FACTOR: f32 = 1.0; 
}

/// Common
impl Global {
    /// The overall background, mostly overwritten by later elements (RGBA)
    const BOARD_COLOR: (f32, f32, f32, f32) = (0.1, 0.0, 0.2, 0.5);

    /// The main 10x22 grid background (RGBA)
    const FIELD_COLOR: (f32, f32, f32, f32) = (0.2, 0.2, 0.2, 0.5);

    /// The grid lines - if they are displayed at all (RGBA)
    const GRID_COLOR: (f32, f32, f32, f32) = (0.2, 0.5, 0.2, 0.5);

    /// The borders around the field colour (RGBA)
    const BORDER_COLOR: (f32, f32, f32, f32) = (1.0, 1.0, 1.0, 0.4);

    /// The border size in pixels
    const BORDER_SIZE: f32 = 6.0;

    /// Width of the playing grid in blocks
    const FIELD_WIDTH: i32 = 10;

    /// Height of the playing grid in blocks
    const FIELD_HEIGHT: i32 = 20;

    /// Size of each block in pixels
    const BLOCK_SIZE: f32 = 16.0;

    /// Where the tetrominos start relative to the field and tetronimo size
    const START_POS: (i32, i32) = (4, 4);

    /// The score label 'Score:' (RGBA)
    const SCORELABEL_COLOR: (f32, f32, f32, f32) = (1.0, 1.0, 1.0, 1.0);

    /// The score text (RGBA)
    const SCORE_COLOR: (f32, f32, f32, f32) = (1.0, 0.50, 1.0, 1.0);

    /// The size of the score block
    const SCORE_SIZE: (f32, f32) = (100.0, 25.0);

    /// relative position of the score block from middle.right of field (in blocks)
    const SCORE_SPACE: (f32, f32) = (2.0, -5.0);

    /// Size of the status label in pixels
    const STATUSLABEL_SIZE: f32 = 50.0;

    /// The status label (Paused / game over etc) (RGBA)
    const STATUSLABEL_COLOR: (f32, f32, f32, f32) = (1.0, 0.5, 0.5, 0.5);

    /// Maximum level we allow
    const MAX_LEVEL: usize = 20;
}


// ========================================
// Components

/// Marker for the timer that automatically drops the active tetromino every x seconds
#[derive(Component)]
struct SoftDropTimer(Timer);

/// Marker for blocks that have moved and need their sprites relocated
#[derive(Component)]
struct UpdateBlock;

/// Marker to trigger game restart
#[derive(Component)]
struct Restart;

/// Marker for text UI elements that need to be removed/recreated when the screen size changes
#[derive(Component)]
struct MobileText;

/// Marker to hold the text type ID for UI elements
#[derive(Component, Debug)]
struct TextType {
    id: TextTypes,
}

/// Identifiers for different text UI elements
#[derive(Debug, Copy, Clone, PartialEq)]
enum TextTypes {
    Score = 1,
    Status = 2,
    Level = 3,
    //TEST = 99,
}
// An enum, because we want to avoid id collisions

// Base entity, everything is made out of blocks
#[derive(Component, Debug, Clone)]
struct Block {
    color: Color,
}

/// The shared game state
#[derive(Component, Debug)]
struct Matrix {
    width: i32,
    full_height: i32, // probably don't need both full_height AND max_ypos
    array_size: usize,
    max_ypos: i32,
    field_width: f32,
    field_height: f32,
    height_offset: f32,
    create: bool,
    active: bool,
    occupation: Vec<i8>, // [(y * width) + x] = occupation (0=open, 1=current, 2=heap)
    score: usize,
    level: usize,
    lines_cleared: usize,
    drop_rows: usize,
    drop_speed: f32,
    falling: bool,
    game_over: bool,
}

/// The block's position within the game field
#[derive(Debug, Component)]
struct MatrixPosition {
    x: i32,
    y: i32,
}

/// The blocks that make up each tetromino.
///
/// The block configuration is defined in the BLOCK_INDICIES const
/// The block coloues are in COLORS (RGB values)
/// The block bounding box sizes are in SIZES
#[derive(Debug, Component)]
struct Tetromino {
    tetromino_type: TetrominoType,
    index: MatrixPosition,
}

/// Marker for blocks of the current tetromino
#[derive(Component)]
struct CurrentTetromino;

/// Marker for blocks on the heap
#[derive(Component)]
struct Heap;

// ========================================
// Structures and Enums

/// The different types of tetromino we can have
#[derive(Copy, Clone, Debug, Component)]
enum TetrominoType {
    I = 0,
    O = 1,
    T = 2,
    S = 3,
    Z = 4,
    L = 5,
    J = 6,
}

/// The blocks within each type of tetromino
/// Initial presentation is 'flat side down' as per guidelines
impl Tetromino {
    const BLOCK_INDICES: [[(i32, i32); 4]; 7] = [
        [
            // line, cyan
            (3, 3),
            (2, 3),
            (1, 3),
            (0, 3),
        ],
        [
            // square, yellow
            (0, 0),
            (0, 1),
            (1, 0),
            (1, 1),
        ],
        [
            // T, purple
            (0, 2),
            (1, 2),
            (2, 2),
            (1, 1),
        ],
        [
            // Z, red
            (0, 2),
            (1, 2),
            (1, 1),
            (2, 1),
        ],
        [
            // S, green
            (2, 2),
            (1, 2),
            (1, 1),
            (0, 1),
        ],
        [
            // L, blue
            (2, 1),
            (0, 2),
            (1, 2),
            (2, 2),
        ],
        [
            // J, orange *
            (0, 2),
            (1, 2),
            (2, 2),
            (0, 1),
        ],
    ];

    /// The colours of each tetromino RGB
    const COLORS: [(f32, f32, f32); 7] = [
        (0.0, 0.7, 0.7),  // line, cyan
        (0.7, 0.7, 0.0),  // square, yellow
        (0.7, 0.0, 0.7),  // T, purple
        (0.7, 0.0, 0.0),  // Z, red
        (0.0, 0.7, 0.0),  // S, green
        (0.0, 0.0, 0.7),  // L, blue
        (0.9, 0.25, 0.0), // J, orange
    ];

    /// The size of the bounding box
    const SIZES: [i32; 7] = [
        4, // line, cyan
        2, // square, yellow
        3, // T, purple
        3, // Z, red
        3, // S, green
        3, // L, blue
        3, // J, orange
    ];

    /// A vector of all the blocks that comprise a given TetrominoType
    fn blocks_from_type(tetromino_type: TetrominoType) -> Vec<(Block, Tetromino)> {
        let type_usize = tetromino_type as usize;
        let color = Tetromino::COLORS[type_usize];

        Tetromino::BLOCK_INDICES[type_usize]
            .iter()
            .map(|index| {
                (
                    Block {
                        color: Color::rgb(color.0, color.1, color.2),
                    },
                    Tetromino {
                        index: MatrixPosition {
                            x: index.0,
                            y: index.1,
                        },
                        tetromino_type,
                    },
                )
            })
            .collect()
    }
}

/// Random distribution of the types
impl Distribution<TetrominoType> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> TetrominoType {
        match rng.gen_range(0, 7) {
            0 => TetrominoType::I,
            1 => TetrominoType::O,
            2 => TetrominoType::T,
            3 => TetrominoType::S,
            4 => TetrominoType::Z,
            5 => TetrominoType::L,
            _ => TetrominoType::J,
        }
    }
}

// ========================================
// Application

/// The main application loop
fn main() {
    let min_height = (Global::BLOCK_SIZE + Global::BLOCK_SPACE) * (Global::FIELD_HEIGHT as f32 + 5.0);

    let mut app = App::new();

    app.insert_resource(bevy::window::WindowDescriptor {
        title: "Tetris".to_string(),
        height: min_height as f32,
        width: 600.0,
        resizable: true,
        ..Default::default()
    })
    .add_plugins(DefaultPlugins)
    .insert_resource(SoftDropTimer(Timer::from_seconds(Global::DROP_SPEED_FACTOR, true))) // start speed
    .add_startup_system(tetris_setup)
    // Stages are: First, Startup, PreUpdate, Update, PostUpdate, Last
    .add_system_to_stage(CoreStage::PostUpdate, spawn_current_tetromino) // Needs to happen seperately from other systems
    .add_system(move_current_tetromino)
    .add_system(update_block_sprites)
    .add_system(resize_window)
    .add_system(restart);

    // Debug hierarchy inspector
    #[cfg(debug_assertions)]
    app.add_plugin(bevy_inspector_egui::WorldInspectorPlugin::new());

    app.run();
}

// ========================================
// Systems

/// Set up the game field and internal resources
fn tetris_setup(mut commands: Commands) {
    // Default camera(s)
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(UiCameraBundle::default());

    // Set up some size values
    let field_width = Global::FIELD_WIDTH as f32 * (Global::BLOCK_SIZE + Global::BLOCK_SPACE) - Global::BLOCK_SPACE;
    let field_height = Global::FIELD_HEIGHT as f32 * (Global::BLOCK_SIZE + Global::BLOCK_SPACE) - Global::BLOCK_SPACE;
    let height_offset = Global::START_POS.1 as f32 * (Global::BLOCK_SIZE + Global::BLOCK_SPACE) / 2.0; // Move the field down this many cells to allow for the block entry area

    let array_size = (Global::FIELD_WIDTH * (Global::FIELD_HEIGHT + Global::START_POS.1)) as usize;
    let mut field_array = Vec::with_capacity(array_size);
    for _x in 0..array_size {
        field_array.push(0);
    }

    // The field resource, block sizes and positions
    let matrix = Matrix {
        width: Global::FIELD_WIDTH,
        full_height: Global::FIELD_HEIGHT + Global::START_POS.1,
        array_size,
        max_ypos: Global::FIELD_HEIGHT + Global::START_POS.1 - 1,
        field_width,
        field_height,
        height_offset,
        create: true,
        active: true,
        occupation: field_array,
        score: 0,
        level: 1,
        lines_cleared: 0,
        drop_rows: 0,
        drop_speed: 1.0,
        falling: false,
        game_over: false,
    };

    // Add the overall background as a sprite, centred in the window (so no transform required)
    commands.spawn().insert_bundle(SpriteBundle {
        sprite: Sprite {
            custom_size: Some(Vec2::new(
                field_width + 2.0 * Global::BORDER_SIZE,
                field_height + 2.0 * Global::BORDER_SIZE + 2.0 * height_offset,
            )),
            color: Color::rgba(Global::BOARD_COLOR.0, Global::BOARD_COLOR.1, Global::BOARD_COLOR.2, Global::BOARD_COLOR.3),
            ..Default::default() // Sprite defaults
        },
        ..Default::default() // Sprite bundle defaults
    });

    // Add the field background as a sprite at the bottom of the overall background
    commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            custom_size: Some(Vec2::new(field_width, field_height)),
            color: Color::rgba(Global::FIELD_COLOR.0, Global::FIELD_COLOR.1, Global::FIELD_COLOR.2, Global::FIELD_COLOR.3),
            ..Default::default() // Sprite defaults
        },
        transform: Transform {
            translation: Vec3::new(0.0, -height_offset, 0.0),
            ..Default::default()
        },
        ..Default::default() // Sprite bundle defaults
    });

    // Add the field border as three sprites (left, right, bottom). No top border
    // - Left border
    commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            custom_size: Some(Vec2::new(Global::BORDER_SIZE, field_height)),
            color: Color::rgba(
                Global::BORDER_COLOR.0,
                Global::BORDER_COLOR.1,
                Global::BORDER_COLOR.2,
                Global::BORDER_COLOR.3,
            ),
            ..Default::default() // Sprite defaults
        },
        transform: Transform {
            translation: Vec3::new(-(field_width + Global::BORDER_SIZE) / 2.0, -height_offset, 0.0),
            ..Default::default()
        },
        ..Default::default() // Sprite bundle defaults
    });

    // - Right border
    commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            custom_size: Some(Vec2::new(Global::BORDER_SIZE, field_height)),
            color: Color::rgba(
                Global::BORDER_COLOR.0,
                Global::BORDER_COLOR.1,
                Global::BORDER_COLOR.2,
                Global::BORDER_COLOR.3,
            ),
            ..Default::default() // Sprite defaults
        },
        transform: Transform {
            translation: Vec3::new((field_width + Global::BORDER_SIZE) / 2.0, -height_offset, 0.0),
            ..Default::default()
        },
        ..Default::default() // Sprite bundle defaults
    });

    // - Bottom border
    commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            custom_size: Some(Vec2::new(field_width + 2.0 * Global::BORDER_SIZE, Global::BORDER_SIZE)),
            color: Color::rgba(
                Global::BORDER_COLOR.0,
                Global::BORDER_COLOR.1,
                Global::BORDER_COLOR.2,
                Global::BORDER_COLOR.3,
            ),
            ..Default::default() // Sprite defaults
        },
        transform: Transform {
            translation: Vec3::new(
                0.0,
                -(field_height + Global::BORDER_SIZE) / 2.0 - height_offset,
                0.0,
            ),
            ..Default::default()
        },
        ..Default::default() // Sprite bundle defaults
    });

    // Grid lines
    if Global::DRAW_GRID {
        for x in 1..Global::FIELD_WIDTH {
            commands.spawn_bundle(SpriteBundle {
                sprite: Sprite {
                    custom_size: Some(Vec2::new(Global::BLOCK_SPACE, field_height)),
                    color: Color::rgba(Global::GRID_COLOR.0, Global::GRID_COLOR.1, Global::GRID_COLOR.2, Global::GRID_COLOR.3),
                    ..Default::default() // Sprite defaults
                },
                transform: Transform {
                    translation: Vec3::new(
                        (field_width + Global::BORDER_SIZE) / 2.0
                            - (x as f32 * (Global::BLOCK_SIZE + Global::BLOCK_SPACE))
                            - Global::BLOCK_SPACE,
                        -height_offset,
                        0.0,
                    ),
                    ..Default::default()
                },
                ..Default::default() // Sprite bundle defaults
            });
        }

        for y in 1..Global::FIELD_HEIGHT {
            commands.spawn_bundle(SpriteBundle {
                sprite: Sprite {
                    custom_size: Some(Vec2::new(field_width, Global::BLOCK_SPACE)),
                    color: Color::rgba(Global::GRID_COLOR.0, Global::GRID_COLOR.1, Global::GRID_COLOR.2, Global::GRID_COLOR.3),
                    ..Default::default() // Sprite defaults
                },
                transform: Transform {
                    translation: Vec3::new(
                        0.0,
                        -(field_height + Global::BORDER_SIZE) / 2.0 - height_offset
                            + (y as f32 * (Global::BLOCK_SIZE + Global::BLOCK_SPACE))
                            + Global::BLOCK_SPACE,
                        0.0,
                    ),
                    ..Default::default()
                },
                ..Default::default() // Sprite bundle defaults
            });
        }
    }

    // Add the score background as a sprite to the right of the main field
    let xpos = field_width / 2.0 + Global::SCORE_SPACE.0 * (Global::BLOCK_SIZE + Global::BLOCK_SPACE) + Global::SCORE_SIZE.0 / 2.0;
    let ypos = Global::SCORE_SPACE.1 * (Global::BLOCK_SIZE + Global::BLOCK_SPACE) - Global::SCORE_SIZE.1 / 2.0;
    commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            custom_size: Some(Vec2::new(Global::SCORE_SIZE.0, Global::SCORE_SIZE.1)),
            color: Color::rgba(
                Global::SCOREFIELD_COLOR.0,
                Global::SCOREFIELD_COLOR.1,
                Global::SCOREFIELD_COLOR.2,
                Global::SCOREFIELD_COLOR.3,
            ),
            ..Default::default() // Sprite defaults
        },
        transform: Transform {
            translation: Vec3::new(
                //(field_width + Global::SCORE_SIZE.0) / 2.0 + Global::SCORE_SPACE.0,
                xpos, //-height_offset + Global::SCORE_SIZE.1 / 2.0 - Global::SCORE_SPACE.1,
                ypos, 0.0,
            ),
            ..Default::default()
        },
        ..Default::default() // Sprite bundle defaults
    });

    // UI components (text elements) are created in resize_window(), so that they can move with the window size

    // Add the specification of the field as a resource
    commands.insert_resource(matrix);
}

/// Spawn a new tetromino, check for completed rows, update the score
fn spawn_current_tetromino(
    mut commands: Commands,
    mut matrix: ResMut<Matrix>,
    mut soft_drop_timer: ResMut<SoftDropTimer>,
    mut heap_query: Query<(
        Entity,
        &mut MatrixPosition,
        &Heap,
        Without<CurrentTetromino>,
    )>, // all the blocks in the heap, must be exclude CurrentTetromino or we get a query conflict
    mut text_query: Query<(&mut Text, &TextType)>,
) {
    // If we don't need to create a block, return early
    if !matrix.create {
        return;
    }
    matrix.create = false;
    matrix.falling = false;
    matrix.drop_rows = 0;

    // Check for full rows on the heap - counting from the bottom
    let mut y = matrix.full_height - 1;
    //let mut first_occupied_row = y; // the higehst (lowest numbered) row that contains a block. Used for adjusting drop speed
    let mut full_rows = 0; // number of rows filled
    while y >= 0 {
        let mut full_row = true;
        for x in 0..matrix.width {
            let address = (matrix.width * y + x) as usize;
            if matrix.occupation[address] != 2 {
                // All the blocks in a row must be on the heap
                full_row = false;
                //break; // We can't break because we have to check for the highest block
            }
        }

        if full_row {
            full_rows += 1;

            // If I am on the row to clear, remove me and move me out of bounds so the field array check ignores me
            // If I am above that row, move me down and mark me for update
            for (entity, mut heap_position, _heap, _current) in heap_query.iter_mut() {
                match heap_position.y.cmp(&y) {
                    Ordering::Equal => {
                        heap_position.x = -1;
                        commands.entity(entity).despawn_recursive();
                    }
                    Ordering::Less => {
                        heap_position.y += 1;
                        commands.entity(entity).insert(UpdateBlock);
                    }
                    Ordering::Greater => {}
                }
            }

            // Then deal with the occupation array
            // - move everything above me down one, only go to row 1, which becomes what's in row 0
            // - only move heap blocks
            for y2 in (1..=y).rev() {
                for x in 0..matrix.width {
                    let address_new = (matrix.width * y2 + x) as usize;
                    let address_old = (matrix.width * (y2 - 1) + x) as usize;
                    matrix.occupation[address_new] = matrix.occupation[address_old];
                }
            }
            // - and make sure the first row is empty (of heap blocks - should never happen)
            for x in 0..matrix.width {
                let address_new = (x) as usize;
                if matrix.occupation[address_new] == 2 {
                    matrix.occupation[address_new] = 0;
                }
            }
        } else {
            y -= 1; // If it wasn't a full row we can scan the next one.
                    // We DON'T do this for full rows because the row we just moved down might be full too
        }
    }

    // If we had any full rows, adjust score, level and gravity
    //
    if full_rows > 0 {
        match full_rows {
            1 => {
                matrix.score += 100 * matrix.level;
            }
            2 => {
                matrix.score += 300 * matrix.level;
            }
            3 => {
                matrix.score += 500 * matrix.level;
            }
            4 => {
                matrix.score += 800 * matrix.level;
            }
            x => {
                matrix.score += x * 300 * matrix.level;
            } // What? more than four shouldn't happen
        }

        // Adjust level (need 10 * level to advance)
        matrix.lines_cleared += full_rows;
        if matrix.lines_cleared >= matrix.level * 10 {
            matrix.level = min(matrix.level + 1, Global::MAX_LEVEL);
            matrix.lines_cleared = 0; // This discards any excess rows over the level threshold - eg from a multi-row clearance. Rules are unclear here.
            matrix.drop_speed =
                (0.8 - ((matrix.level - 1) as f32 * 0.007)).powf((matrix.level - 1) as f32);
            //'Guideline' rule: Time = (0.8-((Level-1)*0.007))^(Level-1)
        }
    }
    // Nintendo scoring:  1=40 * (n + 1),  2=100 * (n + 1), 3=300 * (n + 1), 4=1200 * (n + 1)  where n=level
    // plus 1 point per soft drop space (not level dependent)
    // 'Guideline' scoring:  1=100 * (n + 1),  2=300 * (n + 1), 3=500 * (n + 1), 4=800 * (n + 1)  where n=level
    // plus 1 point per soft drop space (not level dependent, applied in move_current_tetromino() )
    // also spin and combo etc - not implemented

    // Adjust the drop speed depending on the highest occupied row - interpolate between the two timer values
    let timer_speed = Global::DROP_SPEED_FACTOR * matrix.drop_speed;
    soft_drop_timer
        .0
        .set_duration(Duration::from_secs_f32(timer_speed));
    soft_drop_timer.0.reset();
    soft_drop_timer
        .0
        .set_elapsed(Duration::from_secs_f32(timer_speed)); // Tetrominoes drop immediately

    // This block will be omitted from release builds
    #[cfg(debug_assertions)]
    {
        // Check that the occupation array matches the blocks
        // Build an occupation array from the entities
        //let array_size = (matrix.width * (matrix.height + Global::START_POS.1)) as usize;
        let mut test_field_array = Vec::with_capacity(matrix.array_size);
        for _x in 0..matrix.array_size {
            test_field_array.push(0);
        }

        // Heap blocks - we shouldn't have any 'current' blocks at this point
        for (_entity, position, _heap, _current) in heap_query.iter() {
            if position.x >= 0 {
                let address = (matrix.width * position.y + position.x) as usize;
                test_field_array[address] = 2;
            }
        }

        for y in 0..matrix.full_height {
            for x in 0..matrix.width {
                let address = (matrix.width * y + x) as usize;
                if test_field_array[address] != matrix.occupation[address] {
                    println!(
                        "Array mismatch @({},{}) test:{} vs matrix:{}",
                        x, y, test_field_array[address], matrix.occupation[address]
                    );
                }
            }
        }
    }

    // Update the score
    for (mut text, text_type) in text_query.iter_mut() {
        match text_type.id {
            TextTypes::Score => {
                text.sections[1].value = format!(" {:07}", matrix.score);
            }
            TextTypes::Level => {
                text.sections[1].value = format!(" {:02}", matrix.level);
            }
            _ => {}
        }
    }

    // Create a new tetromino
    // TODO: random rotation, random horizontal position?
    let tet_type: TetrominoType = rand::random();
    let blocks = Tetromino::blocks_from_type(tet_type);
    for block in blocks.into_iter() {
        let tetromino_matrix_size = Tetromino::SIZES[block.1.tetromino_type as usize];
        let (xpos, ypos) = grid_position(
            &matrix,
            Global::START_POS.0 + block.1.index.x,
            Global::START_POS.1 - tetromino_matrix_size + block.1.index.y,
        );
        let address = (matrix.width * (Global::START_POS.1 - tetromino_matrix_size + block.1.index.y)
            + (Global::START_POS.0 + block.1.index.x)) as usize;
        matrix.occupation[address] = 1;

        let mut tet = commands.spawn_bundle(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(Global::BLOCK_SIZE, Global::BLOCK_SIZE)),
                color: Color::rgb(block.0.color.r(), block.0.color.g(), block.0.color.b()),
                ..Default::default() // Sprite defaults
            },
            transform: Transform::from_translation(Vec3::new(xpos, ypos, 1.0)),
            ..Default::default() // Sprite bundle defaults
        });

        tet.insert(CurrentTetromino);
        tet.insert(MatrixPosition {
            // the starting position of the BLOCK in the game field - starts in the top buffer
            x: Global::START_POS.0 + block.1.index.x,
            y: Global::START_POS.1 - tetromino_matrix_size + block.1.index.y,
        });
        tet.insert(tet_type);
    }
}

/// React to inputs, moving the current tetromino
#[allow(clippy::too_many_arguments)] // Lots of arguments here, some could be into tuples to make clippy happy
fn move_current_tetromino(
    mut commands: Commands,
    time: Res<Time>,                            // game time
    mut soft_drop_timer: ResMut<SoftDropTimer>, // the automatic drop timer
    keyboard_input: Res<Input<KeyCode>>,
    mut matrix: ResMut<Matrix>, // the shared game state
    mut current_query: Query<(
        Entity,
        &mut MatrixPosition,
        &TetrominoType,
        &CurrentTetromino,
    )>, // our current 'dropping' tetromino
    heap_query: Query<(
        Entity,
        &mut MatrixPosition,
        &Heap,
        Without<CurrentTetromino>,
    )>, // all the blocks in the heap, must exclude CurrentTetromino or we get a query conflict. Only used in dbug builds
    mut text_query: Query<(&mut Text, &TextType)>, // to update the status message Paused/Game over
    mut exit: EventWriter<AppExit>,                // to send AppExit events
) {
    // Tick
    soft_drop_timer
        .0
        .tick(Duration::from_secs_f32(time.delta_seconds()));

    // Find out what we want to do, check if we can, then do it if possible
    let mut desired_x = 0;
    let mut desired_y = 0;
    let mut desired_rot = 0;

    // Move left
    if keyboard_input.just_pressed(KeyCode::J) || keyboard_input.just_pressed(KeyCode::Left) {
        desired_x = -1;
    }

    // Move right
    if keyboard_input.just_pressed(KeyCode::L) || keyboard_input.just_pressed(KeyCode::Right) {
        desired_x = 1;
    }

    // Down - including timed drop
    if keyboard_input.just_pressed(KeyCode::K)
        || keyboard_input.just_pressed(KeyCode::Down)
        || soft_drop_timer.0.just_finished()
    {
        desired_y = 1;
    }

    // Rotate clockwise
    if keyboard_input.just_pressed(KeyCode::X) {
        desired_rot = 1;
    }

    // Rotate anti-clockwise
    if keyboard_input.just_pressed(KeyCode::Z) {
        desired_rot = -1;
    }

    // Drop to bottom
    if keyboard_input.just_pressed(KeyCode::Space) {
        matrix.falling = true;
    }

    // Testing: Print a text version of the internal occupation matrix - it should visually match the block on screen
    #[cfg(debug_assertions)]
    if keyboard_input.just_pressed(KeyCode::Slash) {
        pretty_print(&matrix);
    }

    // Testing: Compare the internal occupation matrix with the block positions
    #[cfg(debug_assertions)]
    if keyboard_input.just_pressed(KeyCode::Apostrophe) {
        println!("Check array");
        // Build an occupation array from the entities
        //let array_size = (matrix.width * (matrix.height + Global::START_POS.1)) as usize;
        let mut test_field_array = Vec::with_capacity(matrix.array_size);
        for _x in 0..matrix.array_size {
            test_field_array.push(0);
        }

        // Current blocks
        for (_entity, position, _tet_type, _current) in current_query.iter() {
            let address = (matrix.width * position.y + position.x) as usize;
            test_field_array[address] = 1;
        }

        // Heap blocks
        for (_entity, position, _heap, _current) in heap_query.iter() {
            let address = (matrix.width * position.y + position.x) as usize;
            test_field_array[address] = 2;
        }

        // Compare
        for y in 0..matrix.full_height {
            for x in 0..matrix.width {
                let address = (matrix.width * y + x) as usize;
                if test_field_array[address] != matrix.occupation[address] {
                    println!(
                        "Array mismatch @({},{}) test:{} vs matrix:{}",
                        x, y, test_field_array[address], matrix.occupation[address]
                    );
                }
            }
        }
    }

    // Quit
    if keyboard_input.just_pressed(KeyCode::Q) {
        exit.send(AppExit);
    }

    // Pause / unpause
    #[allow(clippy::collapsible_if)] // the !matrix.game_over check can be folded into the keycode line, but it seems cleaner to keep all the keycode checks the same
    if keyboard_input.just_pressed(KeyCode::P) || keyboard_input.just_pressed(KeyCode::Escape) {
        if !matrix.game_over {
            matrix.active = !matrix.active;
            soft_drop_timer.0.reset();

            for (mut text, text_type) in text_query.iter_mut() {
                if text_type.id == TextTypes::Status {
                    if matrix.active {
                        text.sections[0].value = "".to_string();
                    } else {
                        text.sections[0].value = "Paused".to_string();
                    }
                }
            }
        }
    }

    // Restart
    if keyboard_input.just_pressed(KeyCode::R) {
        let restart = Restart;
        commands.insert_resource(restart);
    }

    // If the block is falling, that's all we allow, so no steering a falling block
    if matrix.falling {
        desired_y = 1;
        desired_x = 0;
        desired_rot = 0;
        matrix.drop_rows += 1;
    }

    // If we are paused, don't do anything else
    if !matrix.active {
        return;
    }

    // If we don't want to move, don't waste time checking
    if desired_x == 0 && desired_y == 0 && desired_rot == 0 {
        return;
    }

    // Rotation check
    let mut can_rot = true;
    if desired_rot != 0 {
        let mut max_x = 0;
        let mut min_x = matrix.width;
        let mut max_y = 0;
        let mut min_y = matrix.full_height;

        // Find the bounding box of the current entity
        for (_entity, position, _tet_type, _current) in current_query.iter_mut() {
            if position.x < min_x {
                min_x = position.x;
            }
            if position.x > max_x {
                max_x = position.x;
            }
            if position.y < min_y {
                min_y = position.y;
            }
            if position.y > max_y {
                max_y = position.y;
            }
        }

        // Size of the bounds, which give us one of seven shapes
        // These shapes are not the tetrominos themselves, but the shape of the bounding box.
        let size_x = 1 + max_x - min_x;
        let size_y = 1 + max_y - min_y;
        let mut scan_min_x = 0; // Area we have to scan for collisions
        let mut scan_max_x = 0;
        let mut scan_min_y = 0;
        let mut scan_max_y = 0;
        //#[rustfmt::skip] // Much easier to read with horizontal formatting
        match (size_x, size_y, desired_rot) {
            (2, 2, _) => { can_rot = false; } // Square, do nothing

            (1, 4, 1) =>  { scan_min_x = min_x - 1; scan_max_x = max_x + 2; scan_min_y = min_y + 1; scan_max_y = min_y + 1; } //Vbar - rotate +
            (1, 4, -1) => { scan_min_x = min_x - 2; scan_max_x = max_x + 1; scan_min_y = min_y + 1; scan_max_y = min_y + 1; } //Vbar - rotate -

            (4, 1, 1) =>  { scan_min_x = min_x + 1; scan_max_x = min_x + 1; scan_min_y = min_y + 1; scan_max_y = max_y + 2; } //Hbar - rotate +
            (4, 1, -1) => { scan_min_x = min_x - 1; scan_max_x = min_x - 1; scan_min_y = min_y + 1; scan_max_y = max_y + 2; } //Hbar - rotate -

            (2, 3, 1) =>  { scan_min_x = min_x - 1; scan_max_x = min_x + 1; scan_min_y = min_y;     scan_max_y = min_y + 1; } //Vrect - rotate +
            (2, 3, -1) => { scan_min_x = min_x;     scan_max_x = min_x + 2; scan_min_y = min_y;     scan_max_y = min_y + 1; } //Vrect - rotate -

            (3, 2, 1) =>  { scan_min_x = min_x + 1; scan_max_x = min_x + 2; scan_min_y = max_y + 1; scan_max_y = max_y + 1; } //Hrect - rotate +
            (3, 2, -1) => { scan_min_x = min_x;     scan_max_x = min_x + 1; scan_min_y = max_y + 1; scan_max_y = max_y + 1; } //Hrect - rotate -

            (_x, _y, _r) => {} //Unknown
        }

        // Are we trying to rotate over the border?
        if scan_min_x < 0
            || scan_max_x >= matrix.width
            || scan_min_y < 0
            || scan_max_y >= matrix.full_height
        {
            #[cfg(debug_assertions)]
            println!(
                "Rotation Border Collision {:?},{:?} - {:?},{:?} ",
                scan_min_x, scan_min_y, scan_max_x, scan_max_y
            );
            can_rot = false;
        }

        // Check the matrix for any heap collisions if we are still ok
        if can_rot {
            'row_scan: for x in scan_min_x..=scan_max_x {
                for y in scan_min_y..=scan_max_y {
                    let address = (matrix.width * y + x) as usize;
                    if matrix.occupation[address] == 1
                        && (x < min_x || x > max_x || y < min_y || y > max_y)
                    {
                        can_rot = false;
                        break 'row_scan;
                    };
                }
            }
        }

        // Do the rotation
        if can_rot {
            //// Clear the current grid prositions
            //for x in min_x..=max_x {
            //    for y in min_y..=max_y {
            //        let address = (matrix.width * y + x) as usize;
            //        matrix.occupation[address] = 0;
            //    }
            //}

            // Move the blocks
            for (entity, mut position, _tet_type, _current) in current_query.iter_mut() {
                // Clear the current position
                let address = (matrix.width * position.y + position.x) as usize;
                matrix.occupation[address] = 0;
                let (x, y) = rotate_block(
                    position.x,
                    position.y,
                    min_x,
                    min_y,
                    size_x,
                    size_y,
                    desired_rot,
                );
                position.x = x;
                position.y = y;
                // Set the new position
                let address = (matrix.width * position.y + position.x) as usize;
                matrix.occupation[address] = 1;
                commands.entity(entity).insert(UpdateBlock);
            }

            // If we successfully rotate, don't allow horizontal/vertical movement in the same frame, as it seems to confuse the entity query
            //desired_x = 0;
            //desired_y = 0;
        }
    }

    // Horizontal/Vertical movement check
    let mut can_move_x = true; //
    let mut can_move_y = true; // We don't really expect to get an x AND y move in the same frame, but best to be sure.
                               // Scan the heap for collisions - if we want to move vertically or horizontally
    if (desired_x + desired_y) != 0 {
        'my_piece: for (_entity, position, _tet_type, _current) in current_query.iter_mut() {
            // Sidewalls?
            if position.x + desired_x < 0 || position.x + desired_x > matrix.width - 1 {
                can_move_x = false;
            }

            // Bottom?
            if position.y + desired_y > matrix.max_ypos {
                can_move_y = false;
                break 'my_piece; // Can't do anything if we've hit the bottom
            }

            // occupation grid scan - only collide with heap blocks
            if can_move_y && desired_y != 0 {
                let address = (matrix.width * (position.y + desired_y) + (position.x)) as usize;
                if matrix.occupation[address] == 2 {
                    can_move_y = false;
                }
            }

            if can_move_x && desired_x != 0 {
                let address = (matrix.width * (position.y) + (position.x + desired_x)) as usize;
                if matrix.occupation[address] == 2 {
                    can_move_x = false;
                }
            }

            // If we (now) can't move, stop checking
            if !can_move_x && !can_move_y {
                break 'my_piece;
            }
        }

        // If we can move, do so
        if can_move_x || can_move_y {
            for (_entity, position, _tet_type, _current) in current_query.iter_mut() {
                let address = (matrix.width * position.y + position.x) as usize;
                matrix.occupation[address] = 0;
            }
            for (entity, mut position, _tet_type, _current) in current_query.iter_mut() {
                if can_move_x {
                    position.x += desired_x;
                }
                if can_move_y {
                    position.y += desired_y;
                }
                let address = (matrix.width * position.y + position.x) as usize;
                matrix.occupation[address] = 1;
                commands.entity(entity).insert(UpdateBlock);
            }
        }

        // If we want to move down but can't, we must have landed on something, so move this block to the heap and get the next one
        if !can_move_y && desired_y != 0 {
            // If any block is still in the top buffer, we have lost
            for (entity, position, _tet_type, _current) in current_query.iter_mut() {
                if position.y < 4 {
                    matrix.game_over = true;
                    matrix.active = false;
                    //todo: something better?

                    for (mut text, text_type) in text_query.iter_mut() {
                        if text_type.id == TextTypes::Status {
                            text.sections[0].value = "Game over".to_string();
                        }
                    }
                }
                commands.entity(entity).remove::<CurrentTetromino>(); // Remove the component that triggers processing
                commands.entity(entity).insert(Heap); // Put it on the heap
                let address = (matrix.width * position.y + position.x) as usize;
                matrix.occupation[address] = 2;
            }

            // If we were falling, adjust the score
            if matrix.falling {
                matrix.score += matrix.drop_rows - 1; // -1 because we increment this counter before checking for collisions
            }

            // If we haven't lost, trigger the next tetromino
            if !matrix.game_over {
                matrix.create = true;
            }
        }
    }
}

/// Reposition the Updated block sprites based on their grid position
fn update_block_sprites(
    mut commands: Commands,
    matrix: Res<Matrix>,
    mut block_query: Query<(Entity, &MatrixPosition, &mut Transform, &UpdateBlock)>,
) {
    // Move to the new position
    for (entity, position, mut transform, _) in block_query.iter_mut() {
        let (xpos, ypos) = grid_position(&matrix, position.x, position.y);
        commands.entity(entity).remove::<UpdateBlock>();
        let translation = &mut transform.translation;
        translation.x = xpos;
        translation.y = ypos;
    }
}

/// Start a new game
fn restart(
    mut commands: Commands,
    o_matrix: Option<ResMut<Matrix>>,
    restart: Option<Res<Restart>>,
    mut block_query: Query<(Entity, &MatrixPosition, &mut Transform)>,
    mut text_query: Query<(&mut Text, &TextType)>,
) {
    if restart.is_some() && o_matrix.is_some() {
        // Clear the restart flag
        commands.remove_resource::<Restart>();

        // Remove all the blocks
        for (entity, _position, _transform) in block_query.iter_mut() {
            commands.entity(entity).despawn_recursive();
        }

        // Reset the matrix
        #[allow(clippy::unnecessary_unwrap)] 
        let mut matrix = o_matrix.unwrap(); // We have already determined that o_matrix is_some so this will never panic, but we still need to get at the value
        matrix.score = 0;
        matrix.level = 1;
        matrix.lines_cleared = 0;
        matrix.drop_rows = 0;
        matrix.drop_speed = 1.0;
        matrix.active = true;
        matrix.falling = false;
        matrix.create = true; // Triggers a new tetromino and starts the game
        matrix.game_over = false;

        // Clear the occupation array
        //let array_size = (matrix.width * (matrix.height + Global::START_POS.1)) as usize;
        for x in 0..matrix.array_size {
            matrix.occupation[x] = 0;
        }

        // Clear the score and status
        for (mut text, text_type) in text_query.iter_mut() {
            match text_type.id {
                TextTypes::Score => {
                    text.sections[1].value = format!(" {:07}", matrix.score);
                }
                TextTypes::Status => {
                    text.sections[0].value = "".to_string();
                }
                _ => {}
            }
        }
    }
}

/// Recreate some text UI elements when the window resizes to keep them aligned to the game field
fn resize_window(
    mut commands: Commands,
    mut resize_event: EventReader<WindowResized>,
    matrix: ResMut<Matrix>,
    asset_server: Res<AssetServer>,
    mut text_query: Query<(Entity, &mut Text, &TextType, Option<&MobileText>)>,
) {
    let mut do_recreate: bool = false;
    let mut width = 0.0;
    let mut height = 0.0;

    // We seem to get several resie events at a time, so make a note of the last width/height and only update once
    for event in resize_event.iter() {
        width = event.width;
        height = event.height;
        do_recreate = true;
    }

    if do_recreate {
        // remove the text elements that are mobile
        for (entity, _text, _text_type, mobile_text) in text_query.iter_mut() {
            if let Some(_mobile_text) = mobile_text {
                commands.entity(entity).despawn();
            }
        }

        // now recreate them with the new positions
        let font = asset_server.load("fonts/FiraSans-Bold.ttf");

        // the score label and text
        let xpos = (width + matrix.field_width) / 2.0 + Global::SCORE_SPACE.0 * (Global::BLOCK_SIZE + Global::BLOCK_SPACE);
        let ypos = height / 2.0 - (Global::SCORE_SPACE.1 + 1.5) * (Global::BLOCK_SIZE + Global::BLOCK_SPACE); // Note +1.5 here moves the score label UP
        commands
            .spawn_bundle(TextBundle {
                style: Style {
                    align_self: AlignSelf::FlexEnd,
                    position_type: PositionType::Absolute,
                    position: Rect {
                        // Style positions are relative to the window top,left
                        left: Val::Px(xpos),
                        top: Val::Px(ypos),
                        ..Default::default()
                    },
                    ..Default::default()
                },

                text: Text {
                    // Construct a `Vec` of `TextSection`s
                    sections: vec![
                        TextSection {
                            value: "Score: \n".to_string(),
                            style: TextStyle {
                                font: font.clone(),
                                font_size: Global::SCORE_SIZE.1,
                                color: Color::rgba(
                                    Global::SCORELABEL_COLOR.0,
                                    Global::SCORELABEL_COLOR.1,
                                    Global::SCORELABEL_COLOR.2,
                                    Global::SCORELABEL_COLOR.3,
                                ),
                            },
                        },
                        TextSection {
                            value: format!(" {:07}", matrix.score),
                            style: TextStyle {
                                font: font.clone(),
                                font_size: Global::SCORE_SIZE.1,
                                color: Color::rgba(
                                    Global::SCORE_COLOR.0,
                                    Global::SCORE_COLOR.1,
                                    Global::SCORE_COLOR.2,
                                    Global::SCORE_COLOR.3,
                                ),
                            },
                        },
                    ],
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(TextType {
                id: TextTypes::Score,
            })
            .insert(MobileText); // testing

        // the level label and text
        let xpos = (width + matrix.field_width) / 2.0 + Global::SCORE_SPACE.0 * (Global::BLOCK_SIZE + Global::BLOCK_SPACE);
        let ypos = height / 2.0 - (Global::SCORE_SPACE.1 + 3.5) * (Global::BLOCK_SIZE + Global::BLOCK_SPACE); // Note +3.5 here moves the level label UP
        commands
            .spawn_bundle(TextBundle {
                style: Style {
                    align_self: AlignSelf::FlexEnd,
                    position_type: PositionType::Absolute,
                    position: Rect {
                        // Style positions are relative to the window top,left
                        left: Val::Px(xpos),
                        top: Val::Px(ypos),
                        ..Default::default()
                    },
                    ..Default::default()
                },

                text: Text {
                    // Construct a `Vec` of `TextSection`s
                    sections: vec![
                        TextSection {
                            value: "Level: ".to_string(),
                            style: TextStyle {
                                font: font.clone(),
                                font_size: Global::SCORE_SIZE.1,
                                color: Color::rgba(
                                    Global::SCORELABEL_COLOR.0,
                                    Global::SCORELABEL_COLOR.1,
                                    Global::SCORELABEL_COLOR.2,
                                    Global::SCORELABEL_COLOR.3,
                                ),
                            },
                        },
                        TextSection {
                            value: format!(" {:02}", matrix.level),
                            style: TextStyle {
                                font: font.clone(),
                                font_size: Global::SCORE_SIZE.1,
                                color: Color::rgba(
                                    Global::SCORE_COLOR.0,
                                    Global::SCORE_COLOR.1,
                                    Global::SCORE_COLOR.2,
                                    Global::SCORE_COLOR.3,
                                ),
                            },
                        },
                    ],
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(TextType {
                id: TextTypes::Level,
            })
            .insert(MobileText); // testing

        // the status label
        //let window = windows.get_primary_mut().unwrap();
        let xpos = (width - matrix.field_width) / 2.0;
        let ypos = height / 2.0;
        let mut status_text = "";
        if matrix.game_over {
            status_text = "Game over";
        } else if !matrix.active {
            status_text = "Paused";
        }
        commands
            .spawn_bundle(TextBundle {
                style: Style {
                    align_self: AlignSelf::FlexEnd,
                    position_type: PositionType::Absolute,
                    position: Rect {
                        // Style positions are relative to the window top,left
                        left: Val::Px(xpos),
                        top: Val::Px(ypos),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                // Use the `Text::with_section` constructor for single component elements
                text: Text::with_section(
                    status_text,
                    TextStyle {
                        font, // the last use can consume the font, otherwise we need font.clone()
                        font_size: Global::STATUSLABEL_SIZE,
                        color: Color::rgba(
                            Global::STATUSLABEL_COLOR.0,
                            Global::STATUSLABEL_COLOR.1,
                            Global::STATUSLABEL_COLOR.2,
                            Global::STATUSLABEL_COLOR.3,
                        ),
                        //..Default::default()
                    },
                    TextAlignment {
                        horizontal: HorizontalAlign::Center,
                        ..Default::default()
                    },
                ),
                ..Default::default()
            })
            .insert(TextType {
                id: TextTypes::Status,
            })
            .insert(MobileText);
    }
}

// ========================================
// Utility functions

/// Print a text version of the occupation grid. Only used in debug builds
#[cfg(debug_assertions)]
fn pretty_print(matrix: &Matrix) {
    // Higher rows numbers (y) are at the bottom
    for y in 0..matrix.full_height {
        let slice_start = (matrix.width * y) as usize;
        let slice_end = (matrix.width * (y + 1)) as usize; // not included in slice
        let slice = &(matrix.occupation)[slice_start..slice_end];
        println!("occupation {:2} {:?}", y, slice);
    }
}

/// Calculate screen position from the block co-ordinates in the playing grid
fn grid_position(matrix: &Matrix, xpos: i32, ypos: i32) -> (f32, f32) {
    let x =
        -(matrix.field_width) / 2.0 + xpos as f32 * (Global::BLOCK_SIZE + Global::BLOCK_SPACE) + Global::BLOCK_SIZE / 2.0;
    let y = (matrix.field_height) / 2.0 + matrix.height_offset
        - ypos as f32 * (Global::BLOCK_SIZE + Global::BLOCK_SPACE)
        - Global::BLOCK_SIZE / 2.0;

    (x, y)
}

/// Rotate a block within a bounding box
fn rotate_block(
    x: i32,
    y: i32,
    min_x: i32,
    min_y: i32,
    size_x: i32,
    size_y: i32,
    desired_rot: i32,
) -> (i32, i32) {
    let rel_x = x - min_x; // Relative position of the current block within the tetromino bounding box
    let rel_y = y - min_y;

    let mut new_x = x; // Default to no change
    let mut new_y = y;

    // Brute force approach. Aesthetically offensive but not as inefficient as it seems, and much easier to debug
    //#[rustfmt::skip] // Much easier to read with horizontal formatting
                       // Sadly cargo build is unhappy about it at the moment, so we comment it out until the feature is available.
                       
    match (size_x, size_y, desired_rot, rel_x, rel_y) {
        //(2, 2, _) => { } // Shouldn't happen because we avoid rotating squares

        // VBar, rotate+
        (1, 4, 1, 0, 0) => { new_x = x - 1; new_y = y + 1; }
        (1, 4, 1, 0, 1) => { new_x = x;     new_y = y;     }
        (1, 4, 1, 0, 2) => { new_x = x + 1; new_y = y - 1; }
        (1, 4, 1, 0, 3) => { new_x = x + 2; new_y = y - 2; }

        // VBar, rotate-
        (1, 4, -1, 0, 0) => { new_x = x + 1; new_y = y + 1; }
        (1, 4, -1, 0, 1) => { new_x = x;     new_y = y;     }
        (1, 4, -1, 0, 2) => { new_x = x - 1; new_y = y - 1; }
        (1, 4, -1, 0, 3) => { new_x = x - 2; new_y = y - 2; }

        // HBar, rotate+
        (4, 1, 1, 0, 0) => { new_x = x + 1; new_y = y - 1; }
        (4, 1, 1, 1, 0) => { new_x = x;     new_y = y;     }
        (4, 1, 1, 2, 0) => { new_x = x - 1; new_y = y + 1; }
        (4, 1, 1, 3, 0) => { new_x = x - 2; new_y = y + 2; }

        // HBar, rotate-
        (4, 1, -1, 0, 0) => { new_x = x + 2; new_y = y + 2; }
        (4, 1, -1, 1, 0) => { new_x = x + 1; new_y = y + 1; }
        (4, 1, -1, 2, 0) => { new_x = x;     new_y = y;     }
        (4, 1, -1, 3, 0) => { new_x = x - 1; new_y = y - 1; }

        // VRect, rotate-
        (2, 3, -1, 0, 0) => { new_x = x;     new_y = y + 1; }
        (2, 3, -1, 1, 0) => { new_x = x - 1; new_y = y;     }
        (2, 3, -1, 0, 1) => { new_x = x + 1; new_y = y;     }
        (2, 3, -1, 1, 1) => { new_x = x;     new_y = y - 1; }
        (2, 3, -1, 0, 2) => { new_x = x + 2; new_y = y - 1; }
        (2, 3, -1, 1, 2) => { new_x = x + 1; new_y = y - 2; }

        // VRect, rotate+
        (2, 3, 1, 0, 0) => { new_x = x + 1; new_y = y;     }
        (2, 3, 1, 1, 0) => { new_x = x;     new_y = y + 1; }
        (2, 3, 1, 0, 1) => { new_x = x;     new_y = y - 1; }
        (2, 3, 1, 1, 1) => { new_x = x - 1; new_y = y;     }
        (2, 3, 1, 0, 2) => { new_x = x - 1; new_y = y - 2; }
        (2, 3, 1, 1, 2) => { new_x = x - 2; new_y = y - 1; }

        // HRect, rotate+
        (3, 2, 1, 0, 0) => { new_x = x + 2; new_y = y;     }
        (3, 2, 1, 1, 0) => { new_x = x + 1; new_y = y + 1; }
        (3, 2, 1, 2, 0) => { new_x = x;     new_y = y + 2; }
        (3, 2, 1, 0, 1) => { new_x = x + 1; new_y = y - 1; }
        (3, 2, 1, 1, 1) => { new_x = x;     new_y = y;     }
        (3, 2, 1, 2, 1) => { new_x = x - 1; new_y = y + 1; }

        // HRect, rotate-
        (3, 2, -1, 0, 0) => { new_x = x;     new_y = y + 2; }
        (3, 2, -1, 1, 0) => { new_x = x - 1; new_y = y + 1; }
        (3, 2, -1, 2, 0) => { new_x = x - 2; new_y = y;     }
        (3, 2, -1, 0, 1) => { new_x = x + 1; new_y = y + 1; }
        (3, 2, -1, 1, 1) => { new_x = x;     new_y = y;     }
        (3, 2, -1, 2, 1) => { new_x = x - 1; new_y = y - 1; }

        // What?
        (x, y, r, rx, ry) => {
            println!("Unknown {} {} {} {} {}", x, y, r, rx, ry);
        }
    }

    (new_x, new_y)
}

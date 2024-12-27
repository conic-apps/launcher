// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};

/// The gamerules used in the world.
///
/// Rule name: The value for the given rule. This is always an NBT string, which is either true or
/// false for the majority of rules (with it being a number for some other rules, and any arbitrary
/// string for a user-defined rule)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameRules {
    /// Whether advancements should be announced in chat
    pub announce_advancements: Option<String>,

    /// Whether block loot is dropped by all blocks (false) or randomly (true) depending on how far
    /// the block is from the center of a block explosion (e.g. clicking a bed in dimensions other
    /// than the Overworld).
    pub block_explosion_drop_decay: Option<String>,

    /// Whether command blocks should be enabled in-game
    pub command_block_output: Option<String>,

    /// Whether command blocks should notify admins when they perform commands
    pub command_blocks_enabled: Option<String>,

    /// Controls the maximum number of blocks changed when using /clone, /fill, or /fillbiome
    pub command_modification_block_limit: Option<String>,

    /// Whether the server should skip checking player speed when the player is wearing elytra.
    /// Often helps with jittering due to lag in multiplayer.
    pub disable_elytra_movement_check: Option<String>,

    /// Whether raids are disabled.
    pub disable_raids: Option<String>,

    /// Whether the daylight cycle and moon phases progress
    pub do_daylight_cycle: Option<String>,

    /// Whether entities that are not mobs should have drops
    pub do_entity_drops: Option<String>,

    /// Whether fire should spread and naturally extinguish
    pub do_fire_tick: Option<String>,

    /// Whether phantoms can spawn in the nighttime
    pub do_insomnia: Option<String>,

    /// Players respawn immediately without showing the death screen
    pub do_immediate_respawn: Option<String>,

    /// Whether players can craft only those recipes that they have unlocked
    pub do_limited_crafting: Option<String>,

    /// Whether mobs should drop items and experience orbs
    pub do_mob_loot: Option<String>,

    /// Whether mobs should naturally spawn. Does not affect monster spawners.
    pub do_mob_spawning: Option<String>,

    /// Whether patrols can spawn
    pub do_patrol_spawning: Option<String>,

    /// Whether blocks should have drops
    pub do_tile_drops: Option<String>,

    /// Whether wandering traders can spawn
    pub do_trader_spawning: Option<String>,

    /// Whether vines can spread to other blocks. Cave vines, weeping vines, and twisting vines are
    /// not affected.
    pub do_vines_spread: Option<String>,

    /// Whether the weather can change naturally. The /weather command can still change weather.
    pub do_weather_cycle: Option<String>,

    /// Whether wardens can spawn
    pub do_warden_spawning: Option<String>,

    /// Whether the player should take damage when drowning
    pub drowning_damage: Option<String>,

    /// Whether the player should take fall damage
    pub fall_damage: Option<String>,

    /// Whether the player should take damage in fire, lava, campfires, or on magma blocks.
    pub fire_damage: Option<String>,

    /// Makes angered neutral mobs stop being angry when the targeted player dies nearby
    pub forgive_dead_players: Option<String>,

    /// Whether the player should take damage when inside powder snow
    pub freeze_damage: Option<String>,

    /// Whether certain sound events are heard by all players regardless of location
    pub global_sound_events: Option<String>,

    /// The maximum number of commands that can be executed by /function at once
    pub function_command_limit: Option<String>,

    /// Whether the player should keep items and experience in their inventory after death
    pub keep_inventory: Option<String>,

    /// Whether new sources of lava are allowed to form
    pub lava_source_conversion: Option<String>,

    /// Whether to log admin commands to server log
    pub log_admin_commands: Option<String>,

    /// The maximum length of a chain of commands that can be executed during one tick. Applies to
    /// command blocks and functions.
    pub max_command_chain_length: Option<String>,

    /// The maximum number of pushable entities a mob or player can push, before taking 6♥♥♥ entity
    /// cramming damage per half-second. Setting to 0 or lower disables the rule. Damage affects
    /// Survival-mode or Adventure-mode players, and all mobs but bats. Pushable entities include
    /// non-Spectator-mode players, any mob except bats, as well as boats and minecarts.
    pub max_entity_cramming: Option<String>,

    /// Whether block loot is dropped by all blocks (false) or randomly (true) depending on how far
    /// the block is from the center of a mob explosion (e.g. Creeper explosion).
    pub mob_explosion_drop_decay: Option<String>,

    /// Whether creepers, zombies, endermen, ghasts, withers, ender dragons, rabbits, sheep,
    /// villagers, silverfish, snow golems, and end crystals should be able to change blocks, and
    /// whether mobs can pick up items. When mobGriefing is disabled, piglins do not pick up gold
    /// ingots, but a player can still barter with them by using the item on the mob. Similarly,
    /// villagers do not pick up food items but can still breed until they run out of any food
    /// already in their inventory. This also affects the capability of zombie-like creatures like
    /// zombified piglins and drowned to pathfind to turtle eggs.
    pub mob_griefing: Option<String>,

    /// Whether the player can regenerate health naturally if their hunger is full enough (doesn't
    /// affect external healing, such as golden apples, the Regeneration effect, etc.)
    pub natural_regeneration: Option<String>,

    /// What percentage of players in the Overworld must sleep to skip the night. A percentage value
    /// of 0 or less will allow the night to be skipped by just 1 player, and a percentage value
    /// more than 100 will prevent players from ever skipping the night.
    pub players_sleeping_percentage: Option<String>,

    /// Whether the player can fight with other players
    pub pvp: Option<String>,

    /// How often a random block tick occurs (such as plant growth, leaf decay, etc.) per chunk
    /// section per game tick. 0 and negative values disables random ticks, higher numbers increase
    /// random ticks. Setting to a high integer results in high speeds of decay and growth. Numbers
    /// over 4096 make plant growth or leaf decay instantaneous.
    pub random_tick_speed: Option<String>,

    /// Whether the debug screen shows all or reduced information; and whether the effects of
    /// `F3` + `B` (entity hitboxes) and `F3` + `G` (chunk boundaries) are shown.
    pub reduced_debug_info: Option<String>,

    /// Prevents beds/respawn anchors from exploding in other dimensions.
    pub respawn_blocks_explode: Option<String>,

    /// Whether the feedback from commands executed by a player should show up in chat. Also affects
    /// the default behavior of whether command blocks store their output text
    pub send_command_feedback: Option<String>,

    /// Whether border blocks effects are shown
    pub show_border_effect: Option<String>,

    /// Whether the player's coordinates are displayed
    pub show_coordinates: Option<String>,

    /// Whether death messages are put into chat when a player dies. Also affects whether a message
    /// is sent to the pet's owner when the pet dies.
    pub show_death_message: Option<String>,

    /// Hides the "Can place on" and "Can destroy" block lists from item lore, as well as item lock
    /// indicators.
    pub show_tags: Option<String>,

    /// The maximum number of snow layers that can be accumulated on each block
    pub snow_accumulation_height: Option<String>,

    /// The number of blocks outward from the world spawn coordinates that a player spawns in when
    /// first joining a server or when dying without a personal spawnpoint. Has no effect on servers
    /// where the default game mode is Adventure.
    pub spawn_radius: Option<String>,

    /// Whether players in Spectator mode can generate chunks
    pub spectators_generate_chunks: Option<String>,

    /// Whether TNT explodes after activation
    pub tnt_explodes: Option<String>,

    /// Whether block loot is dropped by all blocks (false) or randomly (true) depending on how far
    /// the block is from the center of a TNT explosion.
    pub tnt_explosion_drop_decay: Option<String>,

    /// Makes angered neutral mobs attack any nearby player, not just the player that angered them.
    /// Works best if forgiveDeadPlayers is disabled.
    pub universal_anger: Option<String>,

    /// Whether new sources of water are allowed to form
    pub water_source_conversion: Option<String>,
}

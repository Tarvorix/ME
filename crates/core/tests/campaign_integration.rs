use machine_empire_core::campaign::map::{CampaignMap, GarrisonedUnit, SiteType};
use machine_empire_core::campaign::economy::CampaignEconomy;
use machine_empire_core::campaign::dispatch::DispatchQueue;
use machine_empire_core::campaign::research::{PlayerResearch, TechId, get_modified_blueprint};
use machine_empire_core::campaign::bridge::{BattleRequest, create_battle_from_campaign, extract_battle_result};
use machine_empire_core::campaign::garrison;
use machine_empire_core::campaign_game::CampaignGame;
use machine_empire_core::ai::campaign_ai::CampaignAiDifficulty;

/// Full campaign game with 2 AI players running for many ticks.
/// Verifies the game loop doesn't panic and eventually produces activity.
#[test]
fn test_ai_vs_ai_campaign_game() {
    let mut game = CampaignGame::new(2, 42);
    game.add_ai_player(0, CampaignAiDifficulty::Normal);
    game.add_ai_player(1, CampaignAiDifficulty::Normal);

    // Run for 2000 ticks (100 seconds of game time at 20 ticks/sec)
    for _ in 0..2000 {
        game.tick();
    }

    // Game should have progressed
    assert!(game.tick_count > 0);

    // Economies should have accumulated energy
    let econ0 = &game.economies[0];
    let econ1 = &game.economies[1];
    assert!(econ0.energy_bank > 0.0, "Player 0 should have energy");
    assert!(econ1.energy_bank > 0.0, "Player 1 should have energy");
}

/// Campaign map generation produces valid, playable maps.
#[test]
fn test_campaign_map_generation_is_valid() {
    for seed in 0..5u64 {
        let map = CampaignMap::generate(2, seed);

        // Should have forges for both players
        assert_eq!(map.player_forges.len(), 2);

        // Should have mines and relics
        let mines = map.sites.iter().filter(|s| s.site_type == SiteType::MiningStation).count();
        let relics = map.sites.iter().filter(|s| s.site_type == SiteType::RelicSite).count();

        assert!(mines >= 4, "Should have at least 4 mines, got {}", mines);
        assert!(relics >= 1, "Should have at least 1 relic, got {}", relics);

        // All sites should have valid positions
        for site in &map.sites {
            assert!(site.x >= 0.0 && site.x <= map.width);
            assert!(site.y >= 0.0 && site.y <= map.height);
        }
    }
}

/// Campaign->battle flow: create battle from request, verify game is playable.
#[test]
fn test_campaign_to_battle_flow() {
    let request = BattleRequest {
        site_id: 99,
        attacker: 0,
        defender: 1,
        attacker_forces: vec![GarrisonedUnit::new(0, 5)], // 5 Thralls
        defender_forces: vec![GarrisonedUnit::new(0, 3)], // 3 Thralls
        map_seed: 42,
    };

    // Create battle game
    let mut game = create_battle_from_campaign(&request);

    // Verify battle state is Active
    let bs = game.world.get_resource::<machine_empire_core::systems::battle_victory::BattleState>().unwrap();
    assert!(bs.is_active(), "Battle should start in Active state");

    // Run battle for 200 ticks — should not panic
    for _ in 0..200 {
        game.tick(50.0);
    }

    // Game should have advanced
    assert!(game.tick_count > 0);

    // Both players should have entities (combat + buildings)
    let ut_storage = game.world.get_storage::<machine_empire_core::components::UnitType>().unwrap();
    let mut p0_count = 0u32;
    let mut p1_count = 0u32;
    for (_entity, ut) in ut_storage.iter() {
        if ut.owner == 0 { p0_count += 1; }
        if ut.owner == 1 { p1_count += 1; }
    }
    assert!(p0_count > 0, "Player 0 should have entities");
    assert!(p1_count > 0, "Player 1 should have entities");
}

/// Verify extract_battle_result works when battle is forced to finish.
#[test]
fn test_battle_result_extraction() {
    let request = BattleRequest {
        site_id: 42,
        attacker: 0,
        defender: 1,
        attacker_forces: vec![GarrisonedUnit::new(0, 5)],
        defender_forces: vec![GarrisonedUnit::new(0, 3)],
        map_seed: 42,
    };

    let mut game = create_battle_from_campaign(&request);

    // Run past the minimum tick threshold
    for _ in 0..250 {
        game.tick(50.0);
    }

    // Kill all defender units and buildings to trigger elimination
    let entities_to_kill: Vec<_> = {
        let ut_storage = game.world.get_storage::<machine_empire_core::components::UnitType>().unwrap();
        ut_storage.iter()
            .filter(|(_, ut)| ut.owner == 1)
            .map(|(e, _)| e)
            .collect()
    };

    for entity in entities_to_kill {
        if let Some(h) = game.world.get_component_mut::<machine_empire_core::components::Health>(entity) {
            h.current = 0.0;
        }
    }

    // Tick a few more times for death cleanup + victory check
    for _ in 0..10 {
        game.tick(50.0);
    }

    let result = extract_battle_result(&game, 42, 0, 1);
    assert!(result.is_some(), "Should produce a result after one side eliminated");
    let result = result.unwrap();
    assert_eq!(result.site_id, 42);
    assert_eq!(result.winner, 0);
}

/// Economic spiral: claiming mines increases income over time.
#[test]
fn test_economic_spiral_with_mines() {
    let mut map = CampaignMap::generate(2, 42);

    // Give player 0 some mines
    let mine_ids: Vec<u32> = map.sites.iter()
        .filter(|s| s.is_neutral() && s.site_type == SiteType::MiningStation)
        .take(3)
        .map(|s| s.id)
        .collect();

    // Track economy without mines first
    let mut econs_base = vec![CampaignEconomy::new(), CampaignEconomy::new()];
    for _ in 0..100 {
        machine_empire_core::campaign::economy::campaign_resource_tick(
            &mut econs_base,
            &map,
            0.05,
        );
    }
    let bank_without_mines = econs_base[0].energy_bank;

    // Now give player 0 the mines and track again
    for &mine_id in &mine_ids {
        if let Some(site) = map.get_site_mut(mine_id) {
            site.owner = 0;
        }
    }

    let mut econs_mines = vec![CampaignEconomy::new(), CampaignEconomy::new()];
    for _ in 0..100 {
        machine_empire_core::campaign::economy::campaign_resource_tick(
            &mut econs_mines,
            &map,
            0.05,
        );
    }
    let bank_with_mines = econs_mines[0].energy_bank;

    // With mines should accumulate more
    assert!(bank_with_mines > bank_without_mines,
        "Mines should accelerate economy: with={} vs without={}", bank_with_mines, bank_without_mines);
}

/// Research tech is applied correctly to unit blueprints.
#[test]
fn test_tech_applied_in_battle() {
    let mut research = PlayerResearch::new();

    // Start research (no relic/energy check in start_research — that's in can_research)
    let _cost = research.start_research(TechId::ThrallPlating);

    // Advance research to completion (60s at 0.05s/tick = 1200 ticks, add extra for float safety)
    for _ in 0..1300 {
        research.research_tick(0.05);
    }

    assert!(research.completed.contains(&TechId::ThrallPlating));

    // Get a modified blueprint
    let modified = get_modified_blueprint(
        machine_empire_core::types::SpriteId::Thrall,
        &research.completed,
    );

    let base = machine_empire_core::blueprints::get_blueprint(
        machine_empire_core::types::SpriteId::Thrall,
    );

    // ThrallPlating adds 20% HP
    assert!(modified.max_hp > base.max_hp, "ThrallPlating should increase HP");
    let expected = base.max_hp * 1.2;
    assert!((modified.max_hp - expected).abs() < 0.01);
}

/// Dispatch order delivers units to target site.
#[test]
fn test_dispatch_delivery() {
    let mut map = CampaignMap::generate(2, 42);
    let mut queue = DispatchQueue::new();

    // Find player 0's forge
    let forge_site_id = map.player_forges[0];

    // Find a neutral mine
    let mine = map.sites.iter().find(|s| {
        s.is_neutral() && s.site_type == SiteType::MiningStation
    }).unwrap();
    let mine_id = mine.id;

    // Dispatch some units
    let units = vec![GarrisonedUnit::new(0, 5)]; // 5 Thralls
    let order_id = queue.dispatch_force(&mut map, 0, forge_site_id, mine_id, units);
    assert!(order_id.is_some(), "Dispatch should succeed");

    assert_eq!(queue.orders.len(), 1);

    // Tick until arrival
    for _ in 0..10000 {
        let completed = queue.tick(0.05);
        for order in &completed {
            DispatchQueue::process_arrival(&mut map, order);
        }
        if queue.orders.is_empty() {
            break;
        }
    }

    // Units should have arrived (order completed)
    assert!(queue.orders.is_empty(), "Dispatch order should have completed");

    // Target site should now be claimed by player 0 (neutral claim)
    let mine_after = map.get_site(mine_id).unwrap();
    assert_eq!(mine_after.owner, 0, "Mine should be claimed by player 0");
}

/// Garrison management: add, remove, merge correctly.
#[test]
fn test_garrison_operations() {
    let mut map = CampaignMap::generate(2, 42);
    let forge_site_id = map.player_forges[0];

    // Get forge index
    let forge_idx = map.sites.iter().position(|s| s.id == forge_site_id).unwrap();

    // Initial garrison should have starting forces
    let initial_thrall_count: u32 = map.sites[forge_idx].garrison.iter()
        .filter(|g| g.unit_type == 0)
        .map(|g| g.count)
        .sum();
    assert!(initial_thrall_count > 0, "Forge should start with Thralls");

    // Add more Thralls
    garrison::add_to_garrison(
        &mut map.sites[forge_idx].garrison,
        GarrisonedUnit::new(0, 10),
    );

    // Merged count should be higher
    let new_thrall_count: u32 = map.sites[forge_idx].garrison.iter()
        .filter(|g| g.unit_type == 0)
        .map(|g| g.count)
        .sum();
    assert_eq!(new_thrall_count, initial_thrall_count + 10);

    // Remove some
    let removed = garrison::remove_from_garrison(
        &mut map.sites[forge_idx].garrison,
        0, // unit_type = Thrall
        3,
    );
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().count, 3);
}

/// Full campaign lifecycle: generate, economy, dispatch.
#[test]
fn test_full_campaign_lifecycle() {
    let mut game = CampaignGame::new(2, 42);

    // Player 0 starts with a forge
    let forge_id = game.campaign_map.player_forges[0];
    let forge = game.campaign_map.get_site(forge_id).unwrap();
    assert_eq!(forge.owner, 0);

    // Run economy ticks to accumulate energy
    for _ in 0..200 {
        game.tick();
    }

    // Both players should have energy
    assert!(game.economies[0].energy_bank > 100.0,
        "Player 0 bank: {}", game.economies[0].energy_bank);
    assert!(game.economies[1].energy_bank > 100.0,
        "Player 1 bank: {}", game.economies[1].energy_bank);
}

/// Multiple map seeds produce different but valid maps.
#[test]
fn test_map_variety() {
    let map1 = CampaignMap::generate(2, 1);
    let map2 = CampaignMap::generate(2, 99);

    // Mine positions should differ between seeds
    let mines1: Vec<(f32, f32)> = map1.sites.iter()
        .filter(|s| s.site_type == SiteType::MiningStation)
        .map(|s| (s.x, s.y))
        .collect();
    let mines2: Vec<(f32, f32)> = map2.sites.iter()
        .filter(|s| s.site_type == SiteType::MiningStation)
        .map(|s| (s.x, s.y))
        .collect();

    assert_ne!(mines1, mines2, "Different seeds should produce different mine layouts");
}

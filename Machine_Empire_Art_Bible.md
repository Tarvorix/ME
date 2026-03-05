# Machine Empire — Art Bible & Asset List

**Version:** 0.1
**Date:** March 2, 2026
**Perspective:** Isometric (3/4 view, ~30° camera angle)
**Directions:** 8 (N / NE / E / SE / S / SW / W / NW)
**Render Target:** PixiJS v8 (WebGL2), sprite sheet atlases

---

## 1. Art Direction

### Theme

Machine civil war. All factions use identical units — differentiation is through **player color**, not faction design. The aesthetic is industrial-military: metal, oil, exhaust, glowing optics, worn plating. Not sleek sci-fi — more gritty, utilitarian, battle-scarred.

### Color Language

| Element | Color Approach |
|---------|---------------|
| Player 1 | Red accent (optics, markings, shoulder plates) |
| Player 2 | Blue accent |
| Player 3 | Green accent |
| Player 4 | Yellow/Gold accent |
| Neutral/Unclaimed | Grey / dim white |
| Terrain | Muted earth tones, industrial greys |
| UI elements | Dark chrome, amber/orange highlights |
| Fog of war | Black (unexplored), dark translucent (explored), clear (visible) |

Player color should be applied to **specific regions** of each sprite (glowing elements, shoulder pads, banners, hull markings) — not a full tint. This preserves visual detail while making teams instantly readable.

### Visual Hierarchy

At a glance, a player must be able to distinguish:

```
1. UNIT TYPE — silhouette alone should tell you Thrall vs Sentinel vs Tank
   Thrall:    Small, hunched, moves in groups
   Sentinel:  Taller, broader, stands upright, heavier armor visible
   Hover Tank: Large footprint, hovers off ground, turret visible

2. TEAM — player color accents (not full recolor)

3. HEALTH — small health bar above unit (green → yellow → red)

4. STATUS — selection indicator, move/attack order lines
```

### Sprite Dimensions

| Asset Type | Approximate Size | Notes |
|-----------|-----------------|-------|
| Thrall | 32×32 px | Small footprint, fits in groups |
| Sentinel | 40×48 px | Taller, more imposing |
| Hover Tank | 64×48 px | Wide, heavy silhouette |
| Command Post | 96×96 px | Largest RTS structure |
| Capture Point | 32×32 px | Ground marker |
| Terrain tile | 64×32 px | Standard isometric diamond |
| Campaign map icons | 48×48 px | Forge, mining station, relic site |
| Effects/particles | 16×16 to 32×32 px | Explosions, projectiles |

*All sizes are base resolution. Atlases should be built at 1x and 2x for retina/high-DPI displays.*

---

## 2. Unit Sprites

### 2.1 Thrall (Conscripted Infantry)

**Visual concept:** Small, mass-produced foot soldier. Hunched posture, light armor, glowing optic visor, carries a standard-issue rifle. Looks expendable — worn, minimal decoration. Moves in a quick, jittery way.

**Animations:**

| Animation | Frames | Directions | Total Frames | Notes |
|-----------|--------|------------|-------------|-------|
| Idle | 4 | 8 | 32 | Slight sway, breathing, visor flicker |
| Walk | 6 | 8 | 48 | Quick, shuffling gait |
| Attack | 4 | 8 | 32 | Rifle fire, muzzle flash on frame 2-3 |
| Death | 5 | 1 | 5 | Collapse, visor dims. Single direction (no facing needed) |
| **Thrall Total** | | | **117** | |

### 2.2 Sentinel (Elite Cyborg Infantry)

**Visual concept:** Taller and broader than Thralls. Heavy plated armor, reinforced legs, heavier weapon. Stands upright and confident. Glowing power core visible on chest or back. Moves deliberately — not fast, but purposeful. Looks like it was built for war, not pulled from a factory line.

**Animations:**

| Animation | Frames | Directions | Total Frames | Notes |
|-----------|--------|------------|-------------|-------|
| Idle | 4 | 8 | 32 | Steady stance, power core pulses |
| Walk | 8 | 8 | 64 | Heavier footfalls, deliberate movement |
| Attack | 5 | 8 | 40 | Heavy weapon fire, visible recoil |
| Death | 6 | 1 | 6 | Falls forward, sparks, power core dims |
| **Sentinel Total** | | | **142** | |

### 2.3 Hover Tank (Heavy Armor)

**Visual concept:** Wide, low-profile armored vehicle hovering ~1 unit off the ground. Visible hover glow underneath. Rotating turret on top. Exhaust vents on rear. Hull is scarred and industrial. The hover effect makes it glide smoothly — no wheel or track animation needed, but the hover glow pulses/ripples.

**Animations:**

| Animation | Frames | Directions | Total Frames | Notes |
|-----------|--------|------------|-------------|-------|
| Idle | 4 | 8 | 32 | Hovering in place, slight bob, exhaust shimmer |
| Move | 4 | 8 | 32 | Gliding motion, hover glow trails |
| Attack | 4 | 8 | 32 | Turret fires, barrel flash, slight recoil |
| Death | 8 | 1 | 8 | Explosion, crashes to ground, smoke |
| **Tank Total** | | | **104** | |

### 2.4 Unit Sprite Summary

| Unit | Total Frames | Sprite Sheet Size (est.) |
|------|-------------|------------------------|
| Thrall | 117 | ~256×256 per player color |
| Sentinel | 142 | ~512×256 per player color |
| Hover Tank | 104 | ~512×256 per player color |
| **All units** | **363** | |

*If player color is applied via tinting specific regions at runtime (recommended), only one set of base sprites is needed, not 4.*

---

## 3. Building Sprites

### 3.1 Command Post (RTS Battle Only)

**Visual concept:** Deployable forward base. Industrial, functional — antenna array, reinforced walls, landing pad for incoming reinforcements. Glowing player-color beacon on top. Should look like it was dropped from orbit and unfolded.

| State | Frames | Notes |
|-------|--------|-------|
| Deploying | 6 | Unfold animation when placed |
| Active (idle) | 4 | Ambient: antenna rotates, lights pulse, beacon glows |
| Damaged (50%) | 1 | Static: sparks, cracked plating, dimmer lights |
| Damaged (25%) | 1 | Static: heavy damage, fires, flickering beacon |
| Destroyed | 6 | Collapse/explosion, beacon dies |
| Reinforcement arrival | 4 | Teleport/drop-pod effect on landing pad |
| **Command Post Total** | **22** | |

### 3.2 Forge (Campaign Map)

**Visual concept:** The player's home base. Massive industrial complex — smokestacks, molten glow, assembly lines visible. Should feel imposing and permanent. The "heart" of the empire.

| State | Frames | Notes |
|-------|--------|-------|
| Active (idle) | 6 | Smoke rising, molten glow pulsing, machinery moving |
| Upgrading | 4 | Construction sparks, scaffolding overlay |
| Damaged | 2 | Static states: moderate and heavy damage |
| Destroyed | 6 | Major explosion, collapse |
| **Forge Total** | **18** | |

---

## 4. Campaign Map Assets

### 4.1 Map Icons

These are the strategic-level representations shown on the campaign map. Simpler than RTS sprites — more icon-like, readable at a glance.

| Asset | States | Frames Each | Total | Notes |
|-------|--------|-------------|-------|-------|
| Mining Station (neutral) | 1 | 2 | 2 | Dim, inactive look |
| Mining Station (owned) | 4 (per player) | 2 | 8 | Player color, active glow |
| Mining Station (contested) | 1 | 4 | 4 | Flashing/pulsing alert |
| Relic Site (neutral) | 1 | 3 | 3 | Ancient, mysterious glow |
| Relic Site (owned) | 4 (per player) | 2 | 8 | Player color, research glow |
| Relic Site (contested) | 1 | 4 | 4 | Flashing alert |
| Force marker (small) | 4 (per player) | 1 | 4 | 1-10 units |
| Force marker (medium) | 4 (per player) | 1 | 4 | 11-30 units |
| Force marker (large) | 4 (per player) | 1 | 4 | 31+ units |
| Force marker (moving) | 1 | 4 | 4 | Travel animation (arrow/trail) |
| **Campaign Icons Total** | | | **45** | |

### 4.2 Campaign Map Terrain

The campaign map is a stylized strategic view, not a tile-by-tile grid. It needs:

| Asset | Count | Notes |
|-------|-------|-------|
| Background terrain | 1 | Large hand-painted or tiled wasteland/industrial background |
| No-man's land features | 5-8 | Decorative: craters, ruins, pipelines, debris |
| Path/road connections | 3-4 | Visual lines connecting forge to sites |
| Fog of war overlay | 1 | Semi-transparent layer for unexplored regions |

---

## 5. RTS Battle Map Tiles

### 5.1 Map Specifications

| Property | Value |
|----------|-------|
| Grid size | 64×64 tiles |
| Tile size | 64×32 px (standard isometric diamond) |
| World size | ~4096×2048 px (isometric projection) |
| Viewport coverage (1920×1080) | ~30×33 tiles visible |
| Viewport coverage (iPhone 390×844) | ~6×26 tiles (zoom + pan required) |
| Capture points per map | 3 or 5 (always odd, prevents permanent ties) |
| Entry zones | 2 (opposite sides, per player) |
| Generation | Procedural layout + hand-tuned capture point placement |

### 5.2 Terrain Types

**Phase 1 (now):**

| Type | ID | Walkable | Visual | Notes |
|------|----|----------|--------|-------|
| Open ground | 0 | Yes | Industrial flooring, dirt, concrete | Default terrain |
| Impassable | 1 | No | Walls, debris, structures, cliffs | Blocks all movement and vision |

**Future terrain types (extensible — adding a type = new enum + movement cost entry):**

| Type | ID | Walkable | Movement Effect | Notes |
|------|----|----------|----------------|-------|
| Rough | 2 | Yes | Infantry slowed, hover tanks unaffected | Rubble, broken ground |
| Elevated | 3 | Yes | Vision + damage bonus | High ground |
| Hazard | 4 | No | Damage over time if entered | Toxic pools, lava |
| Cover | 5 | Yes | Damage reduction for units behind | Barricades, low walls |
| Road | 6 | Yes | Movement speed bonus | Paved paths |

### 5.3 Base Terrain Tiles (64×32 px Isometric Diamonds)

#### Open Ground

4 visual variants to prevent tiling repetition. Differences are subtle — slight color variation, cracks, stains, surface wear. Must blend seamlessly when placed adjacent.

| Variant | Description |
|---------|-------------|
| Open A | Clean industrial floor, light grey |
| Open B | Same base, minor cracks |
| Open C | Same base, slight discoloration/stain |
| Open D | Same base, surface wear pattern |

#### Impassable

4 visual variants. Must be immediately obvious as "cannot walk here" — solid, tall, heavy.

| Variant | Description |
|---------|-------------|
| Wall | Solid metal wall segment, industrial |
| Debris pile | Collapsed structure, twisted metal |
| Structure | Intact building/machinery block |
| Rock/cliff | Natural obstacle, heavy and dark |

### 5.4 Edge Transition Tiles

Transition tiles sit at the boundary between open ground and impassable terrain. They make the border look natural instead of a hard pixel edge. Uses a **bitmasked auto-tiling** approach — the renderer checks adjacent tiles and selects the correct transition piece automatically.

```
EDGE PIECES (open ground fading into impassable):

  Straight edges (8):
    N    NE    E    SE    S    SW    W    NW

  Inner corners (4):
    NE-inner   SE-inner   SW-inner   NW-inner

  Outer corners (4):
    NE-outer   SE-outer   SW-outer   NW-outer

  Total transition pieces: 16 per terrain pair
```

| Transition Set | Pieces | Notes |
|---------------|--------|-------|
| Open ↔ Impassable | 16 | Required for Phase 1 |
| Open ↔ Rough | 16 | Future — when rough terrain is added |
| Open ↔ Elevated | 16 | Future — when elevation is added |

**Auto-tile bitmask:** Each tile checks its 8 neighbors. The combination of "same type" vs "different type" neighbors produces a bitmask that maps to the correct transition sprite. Standard technique — PixiJS tilemap renderers support this natively.

### 5.5 Map Objects (Placed on Tiles)

Decorative objects placed on top of base tiles. These add visual variety without affecting gameplay (Phase 1). Future: some objects could provide cover or block vision.

| Object | Variants | Placement | Notes |
|--------|----------|-----------|-------|
| Small debris | 3 | On open ground | Scrap metal, broken pipes |
| Large debris | 2 | On open ground | Wrecked machinery, collapsed walls |
| Floor detail | 3 | On open ground | Oil stains, grating, vents |
| Wall detail | 2 | On impassable | Reinforces wall appearance |
| Ambient effect | 2 | On any | Steam vents, sparking wires |
| **Map Objects Total** | **12** | | |

### 5.6 Capture Points

Capture points are special overlay sprites placed on specific tiles. They're hidden until explored through fog of war.

| State | Frames | Notes |
|-------|--------|-------|
| Hidden (in fog) | 0 | Not rendered |
| Neutral (unclaimed) | 4 | Pulsing white/grey beacon |
| Capturing (in progress) | 4 | Progress ring filling in player color |
| Contested (paused) | 4 | Flashing red, progress frozen |
| Captured (owned) | 2 | Solid player color, steady glow |
| **Capture Point Total** | **14 frames** | |

### 5.7 Entry Zones

Visual indicators showing where each player's forces deploy at battle start.

| Element | Description |
|---------|-------------|
| Zone border | Dashed line or glow marking the deployment area |
| Zone fill | Semi-transparent player color overlay |
| Zone label | "DEPLOY HERE" text (UI overlay, not sprite) |

### 5.8 Tile Sprite Summary

| Category | Count | Notes |
|----------|-------|-------|
| Open ground variants | 4 | Base tiles |
| Impassable variants | 4 | Base tiles |
| Edge transitions | 16 | Open ↔ Impassable |
| Map objects | 12 | Decorative |
| Capture point frames | 14 | All states |
| Entry zone elements | 2 | Per-player color |
| **Tile Sprites Total** | **52** | |

### 5.9 Procedural Map Generation Rules

Battle maps are generated procedurally per site type, with hand-tuned templates controlling capture point placement.

```
GENERATION PROCESS:
  1. Select template based on site type (mining vs relic)
  2. Template defines:
     - Capture point positions (fixed, hand-designed for fairness)
     - Entry zone positions (opposite sides, symmetrical)
     - General terrain distribution (% open vs % impassable)
     - Symmetry axis (mirror for fairness)
  3. Generator fills terrain procedurally within template rules:
     - Open ground as default
     - Impassable clusters placed semi-randomly
     - Ensure pathfinding connectivity (no walled-off sections)
     - Decorative objects scattered on open tiles
  4. Validate:
     - Both entry zones can reach all capture points (A* check)
     - No capture point is unreachable
     - Symmetry is maintained (mirrored for 2-player)
```

**Mining Station templates:** More open terrain, fewer chokepoints. Favors mobile warfare. 3 capture points typical.

**Relic Site templates:** More complex terrain, narrow corridors, defensive positions. 5 capture points typical. Rewards careful positioning over raw numbers.

**Symmetry:** All maps are mirrored along the axis between entry zones. Neither player has a terrain advantage.

---

## 6. Effects & Particles

### 6.1 Combat Effects

| Effect | Frames | Size | Notes |
|--------|--------|------|-------|
| Thrall muzzle flash | 3 | 16×16 | Small, quick flash |
| Sentinel muzzle flash | 3 | 24×24 | Brighter, heavier |
| Hover Tank cannon flash | 4 | 32×32 | Large, screen-shake worthy |
| Bullet trail (infantry) | 2 | 8×8 | Small streak, fast |
| Cannon projectile | 3 | 16×16 | Visible shell in flight |
| Impact (small) | 4 | 16×16 | Sparks on hit — infantry weapons |
| Impact (large) | 6 | 32×32 | Explosion on hit — tank cannon |
| Unit explosion (small) | 6 | 32×32 | Thrall death effect |
| Unit explosion (large) | 8 | 64×64 | Tank/building destruction |
| Hover glow | 4 | 48×16 | Under Hover Tank, pulsing |
| **Combat Effects Total** | **43** | |

### 6.2 Game State Effects

| Effect | Frames | Size | Notes |
|--------|--------|------|-------|
| Reinforcement arrival | 6 | 48×48 | Drop-pod / teleport-in at command post |
| Retreat indicator | 4 | 32×32 | Flashing chevrons pointing away from battle |
| Selection circle | 1 | Per unit size | Highlighted ring under selected units |
| Move order indicator | 2 | 16×16 | Waypoint marker at target location |
| Attack order indicator | 2 | 16×16 | Crosshair at target |
| Rally point flag | 2 | 24×24 | At command post rally point |
| Vision range (debug) | 1 | Variable | Circle overlay, dev mode only |
| **State Effects Total** | **18** | |

---

## 7. UI Assets

### 7.1 HUD Elements

| Element | Description | Notes |
|---------|-------------|-------|
| Tab bar background | Dark chrome strip at top of screen | Holds campaign + battle tabs |
| Tab (campaign) | Map icon + status pip slot | 🗺 icon |
| Tab (battle) | Sword icon + site name + status pip | ⚔ icon |
| Status pip (green) | 8×8 dot | Stable |
| Status pip (yellow) | 8×8 dot, pulsing | Needs attention |
| Status pip (red) | 8×8 dot, pulsing fast | Critical |
| Energy icon | 16×16 | Lightning bolt or battery |
| Energy bar background | Sliced 9-patch | Holds current/income display |
| Strain meter background | Sliced 9-patch | 0-100 gauge |
| Strain fill (healthy) | Green gradient | 0-30 |
| Strain fill (warning) | Yellow gradient | 30-50 |
| Strain fill (danger) | Orange gradient | 50-70 |
| Strain fill (critical) | Red gradient | 70-100 |
| Production queue slot | 40×40 | Holds unit icon + progress bar |
| Unit icon (Thrall) | 32×32 | For production queue and UI panels |
| Unit icon (Sentinel) | 32×32 | |
| Unit icon (Hover Tank) | 32×32 | |
| Unit icon (Command Post) | 32×32 | |
| Minimap frame | Decorative border | Fits bottom-left or bottom-right |
| Selection panel background | 9-patch | Shows selected unit info |

### 7.2 Delegation UI

| Element | Description | Notes |
|---------|-------------|-------|
| Delegate button | Prominent action button | Appears during battle control |
| Take Command button | Prominent action button | Appears during observation mode |
| Stance button (Aggressive) | Icon + label | ⚔ sword icon |
| Stance button (Defensive) | Icon + label | 🛡 shield icon |
| Stance button (Hold Position) | Icon + label | 📍 pin icon |
| Stance button (Harass/Delay) | Icon + label | 🎯 target icon |
| Stance button (Reinforce & Hold) | Icon + label | 🔄 refresh icon |
| Stance button (Retreat) | Icon + label | 🏳 flag icon |
| Observation mode banner | "OBSERVING" text overlay | Semi-transparent across top |
| Reinforcement request notification | Popup with approve/deny | Non-blocking notification |

### 7.3 Campaign UI

| Element | Description | Notes |
|---------|-------------|-------|
| Produce unit buttons | Per unit type | In forge production panel |
| Upgrade forge button | Infrastructure upgrade | Shows cost + time |
| Research panel | Available tech options | Shown when relic site owned |
| Dispatch force panel | Unit selection + target picker | For sending expeditions |
| Garrison panel | Units assigned to defend a site | Shows upkeep cost |
| Pause/Unpause button | Campaign time control | |

---

## 8. Sprite Sheet Organization

### Atlas Strategy

All sprites should be packed into texture atlases no larger than **2048×2048 pixels** (iOS Safari WebGL memory constraint). Recommended atlas grouping:

| Atlas | Contents | Estimated Size |
|-------|----------|---------------|
| `units.png` | All unit animations (Thrall + Sentinel + Tank, all directions) | 2048×2048 |
| `buildings.png` | Command Post + Forge, all states | 1024×512 |
| `terrain.png` | All isometric tiles + map objects | 1024×1024 |
| `effects.png` | All particle effects, muzzle flashes, explosions | 512×512 |
| `ui.png` | HUD elements, icons, buttons, panels | 1024×1024 |
| `campaign.png` | Campaign map icons, force markers, site graphics | 512×512 |

**Total estimated texture memory: ~8-12 MB** (well within iOS Safari limits)

### Sprite Sheet Format

- **Format:** PNG with alpha transparency
- **Companion data:** JSON atlas (TexturePacker / ShoeBox format compatible with PixiJS)
- **Naming convention:** `{unit}_{animation}_{direction}_{frame}.png`
  - Example: `thrall_walk_NE_03.png`
  - Packed into atlas with JSON coordinates

### Retina / High-DPI

- Base sprites at 1x resolution
- Optional 2x atlas for retina displays (`units@2x.png`)
- PixiJS handles resolution switching automatically via `@2x` suffix

---

## 9. Animation Timing

| Animation Type | Frame Duration | Loop? | Notes |
|----------------|---------------|-------|-------|
| Idle | 200ms per frame | Yes | Slow, ambient |
| Walk/Move | 100ms per frame | Yes | Matches movement speed |
| Attack | 80ms per frame | No | Fast, punchy. Returns to idle. |
| Death | 120ms per frame | No | Plays once, corpse stays on last frame briefly |
| Building ambient | 250ms per frame | Yes | Slow, atmospheric |
| Capture point pulse | 150ms per frame | Yes | Noticeable rhythm |
| Muzzle flash | 50ms per frame | No | Very fast, 1-2 frame pop |
| Explosion | 80ms per frame | No | Quick but readable |
| Status pip pulse | 500ms per frame | Yes | Slow blink for attention |

---

## 10. Total Frame Count Summary

| Category | Frames |
|----------|--------|
| Thrall (all animations, 8 dir) | 117 |
| Sentinel (all animations, 8 dir) | 142 |
| Hover Tank (all animations, 8 dir) | 104 |
| Command Post (all states) | 22 |
| Forge (all states) | 18 |
| Campaign map icons | 45 |
| Terrain tiles + transitions | 24 |
| Map objects | 12 |
| Capture points | 14 |
| Entry zone elements | 2 |
| Combat effects | 43 |
| Game state effects | 18 |
| **TOTAL SPRITE FRAMES** | **561** |

*UI elements are mostly 9-patches and vector-style assets, not frame-animated — not counted above.*

---

## 11. Placeholder Art Strategy

For development and playtesting before final art is complete:

| Phase | Art Quality | Approach |
|-------|------------|---------|
| Phase 1 (Foundation) | Colored shapes | Rectangles and circles with player color fills. Thrall = small circle, Sentinel = medium circle, Tank = large rectangle. Enough to test gameplay. |
| Phase 2 (Game Systems) | Basic pixel art | Simple but recognizable silhouettes. 8-direction movement. Placeholder animations (2-3 frames per action). |
| Phase 3 (AI & Server) | Improved placeholders | More animation frames, basic effects. Good enough for AI training and playtesting. |
| Phase 4 (Polish) | Final art | Full sprite sheets with all animations, effects, and UI. |

**Placeholder sprites should match final dimensions** so no layout changes are needed when final art drops in. The sprite sheet JSON atlas format stays the same — only the PNG files change.

---

## 12. Art Production Notes

### What Can Be Generated vs Hand-Made

| Asset Type | Recommendation | Rationale |
|-----------|---------------|-----------|
| Unit sprites | AI-generated base + hand cleanup | Complex animations benefit from AI starting point, human polish for consistency |
| Terrain tiles | Tileable texture generation + hand tweaks | Repetitive patterns are good AI targets |
| Effects/particles | Procedural or hand-made | Simple enough to create programmatically or with small sprite sheets |
| UI elements | Hand-designed in Figma/similar | Must be pixel-perfect and cohesive |
| Campaign map | Hand-painted or AI-assisted background | One-time art piece, worth the investment |
| Icons | Hand-designed | Small, must be extremely clear at tiny sizes |

### Color Palette (Reference)

```
Background/terrain:   #2a2a2a  #3d3d3d  #4a4a4a  (industrial greys)
Metal/armor:          #6b6b6b  #8a8a8a  #a0a0a0  (unit base colors)
Player Red:           #e63946  (accents, optics, markings)
Player Blue:          #457b9d  (accents, optics, markings)
Player Green:         #2a9d8f  (accents, optics, markings)
Player Gold:          #e9c46a  (accents, optics, markings)
Energy/UI glow:       #f4a261  #e76f51  (amber/orange)
Health green:         #52b788
Health yellow:        #f4d35e
Health red:           #e63946
Fog unexplored:       #000000  (full black)
Fog explored:         #1a1a1a  @ 70% opacity
Capture neutral:      #ffffff  @ 50% opacity
```

*All colors are starting references — will be refined during art production.*
